//! src/routes/home/mod.rs

use actix_web::{http::header::ContentType, HttpResponse};

pub async fn home() -> HttpResponse {
    // HttpResponse::Ok().finish()
    // `include_str! reads file at path and return `&'static str` at compile time
    // Yes, file contents become stored into binary!
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("home.html"))
}
