use actix_web::{web, HttpResponse};
use chrono::Utc;
// use sqlx::PgConnection;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

/// Actix-Web calls Form::from_request() on our arguments.
/// It tries to deserialise the body into FormData.
/// If it succeeds, it invokes our `subscribe()` function and carries on...
/// Else, it automagically returns 400 Bad Request.
pub async fn subscribe(
    form: web::Form<FormData>,
    // recieving connection from application state!
    pool: web::Data<PgPool>,
) -> HttpResponse {
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
    let query_span = tracing::info_span!("Saving new subscriber details in database.",);
    // This returns a Result that must be used!
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        values($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    // Using `get_ref()` to only fetch an immutable reference from our `web::Data` wrapper.
    .execute(pool.get_ref())
    // Attach the instrumentation, then await.
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            // instrument and query_span remove need for log
            // tracing::info!(
            //     "request_id {} - New Subscriber details have been saved.",
            //     request_id
            // );
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            // Note using std::fmt::Debug format for error
            // error log falls outside query_span
            tracing::error!("Failed to execute query: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
