//! src/routes/login/post.rs
use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::error_chain_fmt;
// for redirecting...
use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::{web, ResponseError};
use secrecy::Secret;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authenication Failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let encoded_error = urlencoding::Encoded::new(self.to_string());
        HttpResponse::build(self.status_code())
            .insert_header((LOCATION, format!("/login?error={}", encoded_error)))
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
}

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
) -> Result<HttpResponse, LoginError> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    // -- Sort of pulling error handling out.
    // match validate_credentials(credentials, &pool).await {
    //     Ok(user_id) => {
    //         tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    //         // New response for HTTP 303 status code
    //         HttpResponse::SeeOther()
    //             .insert_header((LOCATION, "/"))
    //             .finish()
    //     }
    //     Err(_) => {
    //         todo!()
    //     }
    // }
    // HttpResponse::Ok().finish()
    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())
}
