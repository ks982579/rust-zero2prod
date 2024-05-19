//! src/routes/login/post.rs
use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::error_chain_fmt;
// for redirecting...
use actix_web::http::header::LOCATION;
// use actix_web::http::StatusCode;
use actix_web::HttpResponse;
// use actix_web::ResponseError;
use crate::startup::HmacSecret;
use actix_web::error::InternalError;
use actix_web::web;
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication Failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

/* This is deleted when we reworked the login function to have a secret
impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let query_string = format!("error={}", urlencoding::Encoded::new(self.to_string()));

        let secret: &[u8] = todo!();
        // let encoded_error = urlencoding::Encoded::new(self.to_string());
        let hmac_tag = {
            let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
            mac.update(query_string.as_bytes());
            mac.finalize().into_bytes()
        };
        HttpResponse::build(self.status_code())
            // append hex HMAC tag to query params
            .insert_header((
                LOCATION,
                format!("/login?error={query_string}&tag={hmac_tag:x}"),
            ))
            .finish()
    }
    // fn status_code(&self) -> StatusCode {
    //     match self {
    //         LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
    //         LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
    //     }
    // }
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }
} */

#[tracing::instrument(
    skip_all,
    fields(
        username=tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    // secret: web::Data<HmacSecret>,
    // ) -> Result<HttpResponse, LoginError> {
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    // let user_id = validate_credentials(credentials, &pool)
    //     .await
    //     .map_err(|e| match e {
    //         AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
    //         AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
    //     })?;

    // tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            // New response for HTTP 303 status code
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            // let query_string = format!("error={}", urlencoding::Encoded::new(e.to_string()));
            // let hmac_tag = {
            //     let mut mac =
            //         Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes())
            //             .unwrap();
            //     mac.update(query_string.as_bytes());
            //     mac.finalize().into_bytes()
            // };
            let response = HttpResponse::SeeOther()
                .insert_header((
                    LOCATION,
                    // format!("/login?{}&tag={:x}", query_string, hmac_tag),
                    "/login",
                ))
                .insert_header(("Set-Cookie", format!("_flash={e}")))
                .finish();
            Err(InternalError::from_response(e, response))
        }
    }
}
