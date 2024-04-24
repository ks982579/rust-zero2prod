//! src/routes/newsletters.rs
use actix_web::HttpResponse;

pub async fn publish_newletter() -> HttpResponse {
    HttpResponse::Ok().finish()
}
