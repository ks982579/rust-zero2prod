use actix_web::{web, HttpResponse};
use chrono::Utc;
// use sqlx::PgConnection;
use sqlx::PgPool;
// use tracing::Instrument;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

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
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'confirmed')
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
        // Using `?` will return early if failed with `sqlx::Error`
    })?;
    Ok(())
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

/// Actix-Web calls Form::from_request() on our arguments.
/// It tries to deserialise the body into FormData.
/// If it succeeds, it invokes our `subscribe()` function and carries on...
/// Else, it automagically returns 400 Bad Request.
#[tracing::instrument(
    // specify message associated to span - default = function_name
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    // recieving connection from application state!
    pool: web::Data<PgPool>,
) -> HttpResponse {
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
    let new_subscriber: NewSubscriber = match form.0.try_into() {
        Ok(form) => form,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    // This returns a Result that must be used!
    // the book passes in `&pool` which might have some hidden dereferencing.
    match insert_subscriber(pool.get_ref(), &new_subscriber).await {
        Ok(_) => {
            /* --------------------------------------------------
            * instrument and query_span remove need for log
            tracing::info!(
                "request_id {} - New Subscriber details have been saved.",
                request_id
            );
            ------------------------------------------------ */
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            // dbg!(e);
            // Note using std::fmt::Debug format for error
            // error log falls outside query_span
            // tracing::error!("Failed to execute query: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
