//! src/routes/subscriptions_confirm.rs

use actix_web::{web, HttpResponse};

/// Defines all query parameters we _expect_ to see in incoming request.
/// requires `Deserialize` for actix-web to build it.
#[derive(serde::Deserialize)]
pub struct Parameters {
    // Use `Optional` for optional parameters!
    subscription_token: String,
}

/// The `_parameters` is now an expected query parameter
#[tracing::instrument(name = "Confirm a pending subscriber", skip_all)]
pub async fn confirm(_parameters: web::Query<Parameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
