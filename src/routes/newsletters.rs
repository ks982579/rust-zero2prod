//! src/routes/newsletters.rs
use actix_web::{
    http::{
        header,
        header::{HeaderMap, HeaderValue},
        StatusCode,
    },
    web, HttpRequest, HttpResponse, ResponseError,
};
use anyhow::Context;
use base64::Engine;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt};

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication Failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// Custom implementation of Debug to use `error_chain_fmt()`.
impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

/// To use as HttpResponse...
impl ResponseError for PublishError {
    /*
    fn status_code(&self) -> StatusCode {
        match self {
            // if AuthError, return 401 Unauthorized
            PublishError::AuthError(_) => StatusCode::UNAUTHORIZED,
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    */
    fn error_response(&self) -> HttpResponse {
        match self {
            // if AuthError, return 401 Unauthorized
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// We limit to one field (now) because less work for database
/// and less data over the wire.
struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

/// To get confirmed subscriber emails from database
/// We lean on `anyhow::Error` here to transform sqlx::Error types.
#[tracing::instrument(name = "Get confirmed Subscribers", skip_all)]
async fn get_confirmed_subscribers(
    pool: &PgPool,
    // returning a vector of results to bubble up email errors
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    // Nesting a struct clearly indicates coupling with function
    struct Row {
        email: String,
    }
    // query_as! takes in a struct that it tries to return
    let rows: Vec<Row> = sqlx::query_as!(
        Row,
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#
    )
    .fetch_all(pool)
    .await?;

    // Map into domain type
    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}

#[derive(Debug, serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Debug, serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

/// Using base64 for authentication
fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    // Header value, if present, must be valid UTF8
    let header_value: &str = headers
        .get("Authorization")
        // `context` adds a message to the error
        .context("The 'Authorization' header was missing.")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;

    let base64encoded_segment: &str = header_value
        // Returns Option<&str> without prefix
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;

    let decoded_bytes: Vec<u8> = base64::engine::general_purpose::STANDARD
        // returns Result<Vec<u8>, base64::DecodeError>
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;

    let decoded_credentials: String = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    // Split based on ':' delimiter
    // SplitN is an iterator
    let mut credentials: std::str::SplitN<'_, char> = decoded_credentials.splitn(2, ':');
    // Pull out first value in iterator
    let username: String = credentials
        .next()
        // `ok_or()` is _eagerly_ evaluated, this takes a closure
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    // Pull out second value in iterator
    let password: String = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let user_id: Option<_> = sqlx::query!(
        r#"
    SELECT user_id
    FROM users
    WHERE username = $1 AND password = $2
    "#,
        credentials.username,
        credentials.password.expose_secret()
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")
    .map_err(PublishError::UnexpectedError)?;

    user_id
        .map(|row| row.user_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid Username or Password."))
        .map_err(PublishError::AuthError)
}

/// Pulling `PgPool` from application state.
/// Adding tracing to see who is Calling POST /newsletters
#[tracing::instrument(
    name = "Publish a newsletter issue."
    skip_all
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    // New Extractor!
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers())
        // Bubble up error and convert.
        .map_err(PublishError::AuthError)?;

    // Add username to trace
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, pool.as_ref()).await?;

    // Add user_id to trace
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    // adding the body, good to test sending invalid data
    let subscribers = get_confirmed_subscribers(pool.as_ref()).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    // Similar to `.context()` but with-context is lazy.
                    // The closure is only called when error occurs,
                    // you can avoid paying the cost traveling the error path if unnecessary.
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                // record error chain as structured field on log record
                tracing::warn!(
                        error.cause_chain = ?error,
                        "Skipping a confirmed subscriber.\
                        Their stored contact details are invalid."
                );
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}
