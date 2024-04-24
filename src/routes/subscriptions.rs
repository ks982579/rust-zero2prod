use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng, Rng,
};
// use sqlx::PgConnection;
use sqlx::{Executor, PgPool, Postgres, Transaction};
// use tracing::Instrument;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(Debug)]
pub enum SubscribeError {
    ValidationError(String),
    DatabaseError(sqlx::Error),
    StoreTokenError(StoreTokenError),
    SendEmailError(reqwest::Error),
}

// To use the `?` operator we need the `From<err>` trait
impl From<reqwest::Error> for SubscribeError {
    fn from(value: reqwest::Error) -> Self {
        Self::SendEmailError(value)
    }
}
impl From<sqlx::Error> for SubscribeError {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseError(value)
    }
}
impl From<StoreTokenError> for SubscribeError {
    fn from(value: StoreTokenError) -> Self {
        Self::StoreTokenError(value)
    }
}
// Yes, even the `String`
impl From<String> for SubscribeError {
    fn from(value: String) -> Self {
        Self::ValidationError(value)
    }
}

impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to create a new subscriber.")
    }
}

impl std::error::Error for SubscribeError {}

impl ResponseError for SubscribeError {}

// New error type, wrapping `sqlx::Error`
pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // This struct has text, and the one it wraps also does.
        // write!(f, "{}\nCaused by:\n\t{}", self, self.0)
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Compiler casts `&sqlx::Error` into `dyn Error`
        Some(&self.0)
    }
}

impl ResponseError for StoreTokenError {}

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// Implementing `TryFrom` automagically give you `TryInto`
impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

/// This function takes care of database logic with no awareness of surrounding web framework.
/// Excelent separation of concerns.
#[tracing::instrument(name = "Saving new subscriber details in the database.", skip_all)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );
    // Requires the `sqlx::Executor` trait to be in scope
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
        // Using `?` will return early if failed with `sqlx::Error`
    })?;
    Ok(subscriber_id)
}

/// Returns `true` if input satisfies all our validation constraints
/// on subscriber names, `false` otherwise.
pub fn is_valid_name(s: &str) -> bool {
    // `.trim()` returns a view over the input without trailing
    // whitespace-like characters.
    let is_empty_or_whitespace = s.trim().is_empty();

    let is_too_long = s.graphemes(true).count() > 256;
    let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', ';', '{', '}'];
    let contains_forbidden_characters: bool = s.chars().any(|g| forbidden_characters.contains(&g));

    !(is_empty_or_whitespace || is_too_long || contains_forbidden_characters)
}

/// For parsing subscribers
pub fn parse_subscriber(form: FormData) -> Result<NewSubscriber, String> {
    let name = SubscriberName::parse(form.name)?;
    let email = SubscriberEmail::parse(form.email)?;
    Ok(NewSubscriber { email, name })
}

/// Sending confirmation email when new user registers.
#[tracing::instrument(name = "Send a confirmation email to new subscriber.", skip_all)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format! {
        "{}/subscriptions/confirm?subscription_token={}",
        base_url,
        subscription_token
    };
    // The book does it slightly differently...
    // Send useless email ATM ignoring email delivery errors.
    let greeting = format!("Hi {},", new_subscriber.name.inner_ref());
    let plain_body = format!(
        "Welcome to our newletter!\nPlease visit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newletter!<br/> \
        Click <a href=\"{}\"</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, &greeting, &html_body, &plain_body)
        .await
}

#[tracing::instrument(name = "Store subscription token in the database", skip_all)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
    VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id,
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}

/// Actix-Web calls Form::from_request() on our arguments.
/// It tries to deserialise the body into FormData.
/// If it succeeds, it invokes our `subscribe()` function and carries on...
/// Else, it automagically returns 400 Bad Request.
#[tracing::instrument(
    // specify message associated to span - default = function_name
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    // recieving connection from application state!
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    /* ------------------- Handled in Macro ------------------
    // Generate random unique identifier
    let request_id = Uuid::new_v4();
    // Improving observability
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );
    // Don't actually use `enter` in async function, it is bad.
    let _request_span_guard = request_span.enter();
    // Don't have to call `.enter()` on query_span! because `.instrument` takes care of it.
    let query_span = tracing::info_span!("Saving new subscriber details in database.");
    ----------------------------------------------------- */
    // `form.0` gives access to `FormData`, since `web::Form` is just a wrapper.
    // Can also try `NewSubscriber::try_from(form.0)`.
    let new_subscriber: NewSubscriber = form.0.try_into()?;
    /*  // because of ?, we can remove these for now
        Ok(form) => form,
        Err(_) => return Ok(HttpResponse::BadRequest().finish()),
    };
    */
    // Adding transaction to protect database.
    // We create in parent function and pass down
    let mut transaction = pool.begin().await?;
    /* Because this emits sqlx::Error, and that now implements From
    * we have simple way to transition
        Ok(transaction) => transaction,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };
    */
    // This returns a Result that must be used!
    // the book passes in `&pool` which might have some hidden dereferencing.
    // let subscriber_id = match insert_subscriber(pool.get_ref(), &new_subscriber).await {
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber).await?;
    /* Old way
    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => {
            // dbg!(e);
            // Note using std::fmt::Debug format for error
            // error log falls outside query_span
            // tracing::error!("Failed to execute query: {:?}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };
    */
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token).await?;

    // Commit Database transaction (queries)
    transaction.commit().await?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await?;
    Ok(HttpResponse::Ok().finish())
}

/// Generate a random 25-characters-long case-sensitive subscription token.
fn generate_subscription_token() -> String {
    // let mut rng = thread_rng();
    // (0..25).map(|_| rng.sample(Alphanumeric) as char).collect()
    Alphanumeric.sample_string(&mut rand::thread_rng(), 25)
}

/// Iterators over whole chain of errors
/// Can be used in `Debug` implementation for Error types!
fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    // This writes into a _buffer_
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
