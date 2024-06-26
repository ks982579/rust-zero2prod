//! src/authentication/password.rs
//! Copied everything from old src/authentication.rs
use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid Credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate Credentials", skip_all)]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    // let hasher = Argon2::new(
    //     Algorithm::Argon2id,
    //     Version::V0x13,
    //     Params::new(15000, 2, 1, None)
    //         .context("Failed to build Argon2 parameters")
    //         .map_err(PublishError::UnexpectedError)?,
    // );

    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, &pool).await?
    // .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    /* Upgrading to the above logic...
    let (user_id, expected_password_hash) = get_stored_credentials(&credentials.username, &pool)
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username.")))?;
    */

    // let (expected_password_hash, user_id) = match row {
    //     Some(row) => (row.password_hash, row.user_id),
    //     None => {
    //         return Err(PublishError::AuthError(anyhow::anyhow!(
    //             "Unknown Username."
    //         )));
    //     }
    // };

    /* Replaced with function but keeping notes...
    // Spans kind of stick to a thread, So...
    let current_span: tracing::Span = tracing::Span::current();

    // Confirmed to be CPU intense, we want to move into different thread.
    tokio::task::spawn_blocking(move || {
        // Pass ownership of Span into closure to execute our computation
        // within its scope
        current_span.in_scope(|| {
            // Logic moved into function so this closure can own all necessary data.
            // Else, main thread can drop values and create dangling references.
            verify_password_hash(expected_password_hash, credentials.password)
        })
    })
        */
    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    // spawn_blocking is fallible - we have nested result
    .context("Failed to spawn blocking task.")??;
    // .map_err(PublishError::UnexpectedError)??;
    // The below only set to `Some` if credentials are found in DB.
    // So, if default password somehow matches provided,
    // Still won't authenticate non-existing user (Should add test?).
    // user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username.")))
    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown Username."))
        .map_err(AuthError::InvalidCredentials)
    // Ok(user_id)
}

/// `PasswordHash` has a lifetime, so we must move ownership of
/// its string into thread closure for abide by borrow checker.
/// This function takes ownership of that string.
#[tracing::instrument(name = "Verify Password Hash", skip_all)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;
    // .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid Password.")
        .map_err(AuthError::InvalidCredentials)

    /* Before moving into own function, we can do this for info...
    // Confirmed to be CPU intense, we want to move into different thread.
    tracing::info_span!("Verify password hash").in_scope(|| {
        Argon2::default().verify_password(
            credentials.password.expose_secret().as_bytes(),
            &expected_password_hash,
        )
    })
    .context("Invalid Password.")
    .map_err(PublishError::AuthError)?;
    */
}
/// Extracing db-query logic into own function to give own span.
#[tracing::instrument(name = "Get Stored Credentials", skip_all)]
pub async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")?
    // Just mapping the actual _row_ to a tuple to return
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
}

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password(
    user_id: uuid::Uuid,
    password: Secret<String>,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database.")?;
    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();
    Ok(Secret::new(password_hash))
}
