//! src/routes/subscriptions_confirm.rs

use actix_web::HttpResponse;

#[tracing::instrument(name = "Confirm a pending subscriber", skip_all)]
pub async fn confirm() -> HttpResponse {
    HttpResponse::Ok().finish()
}
