//! src/routes/newsletters.rs
use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt};

#[derive(thiserror::Error)]
pub enum PublishError {
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
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
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

/// Pulling `PgPool` from application state.
pub async fn publish_newletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
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
