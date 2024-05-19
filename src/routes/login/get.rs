//! src/routes/login/get.rs
// use crate::startup::HmacSecret;
use actix_web::cookie::{time::Duration, Cookie};
use actix_web::{http::header::ContentType, web, HttpRequest, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;
// use hmac::{Hmac, Mac};
// use secrecy::ExposeSecret;

// -- related to HMAC
// #[derive(serde::Deserialize)]
// pub struct QueryParams {
//     error: String,
//     tag: String,
// }

// -- related to HMAC
// impl QueryParams {
//     fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
//         let tag = hex::decode(self.tag)?;
//         let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));
//         let mut mac =
//             Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
//         mac.update(query_string.as_bytes());
//         mac.verify_slice(&tag)?;
//         Ok(self.error)
//     }
// }

/* -- Related to HMAC
/// Making query param option instead of its components.
/// Make illegal state impossible to represent using Rust type system.
pub async fn login_form(
    query: Option<web::Query<QueryParams>>,
    secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error_html = match query {
        None => "".into(),
        Some(query) => match query.0.verify(&secret) {
            Ok(error) => {
                format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error))
            }
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify query parameters using the HMAC tag"
            );
                "".into()
            }
        }
        // Some(query) => format!(
        //     "<p><i>{}</i></p>",
        //     htmlescape::encode_minimal(&query.0.error)
        // ),
    };
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!("{}", error_html))
} */

// don't need to access the HttpRequest with FlashMessages...
// pub async fn login_form(request: HttpRequest) -> HttpResponse {
pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();
    // -- Update to display all messages instead of just errors.
    // for m in flash_messages.iter().filter(|m| m.level() == Level::Error) {
    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }
    // -- removed with FlashMessages
    // let error_html: String = match request.cookie("_flash") {
    //     None => "".into(),
    //     Some(cookie) => {
    //         format!("<p><i>{}</i></p>", cookie.value())
    //     }
    // };
    HttpResponse::Ok()
        .content_type(ContentType::html())
        // .cookie(Cookie::build("_flash", "").max_age(Duration::ZERO).finish())
        .body(format!(
            r#"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Login</title>
    <!-- <link href="css/style.css" rel="stylesheet"> -->
  </head>
  <body>
    {error_html}
    <!-- To send a POST request to this endpoint -->
    <form action="/login" method="post">
      <label for="username">
        <input type="text" name="username" placeholder="Enter Username" />
      </label>
      <label for="password">
        <input type="password" name="password" placeholder="Enter Password" />
      </label>
      <button type="submit">Login</button>
    </form>
  </body>
</html>
"#,
        ))
}
