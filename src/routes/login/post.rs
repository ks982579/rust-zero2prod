//! src/routes/login/post.rs
use actix_web::HttpResponse;

pub async fn login() -> HttpResponse {
    HttpResponse::Ok().finish()
}