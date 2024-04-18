//! tests/api/subscriptions.rs

use crate::helpers::{spawn_app, TestApp};
use reqwest::{Client, Response};

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let test_app: TestApp = spawn_app().await;
    // We want to connect to the database also
    // let configuration: Settings = get_configuration().expect("Failed to read configuration.");
    // let connection_string: String = configuration.database.connection_string();
    // Note: `Connection` trait must be in scope to invoke
    // `PgConnection::connect` - it is not an inherent method of the struct!
    // Also, the return type of `.connect()` is wild...
    // let mut connection: PgConnection = PgConnection::connect(&connection_string)
    //     .await
    //     .expect("Failed to connect to Postgres");
    let body: &str = "name=le%20guin&email=ursula_le_guin%40example.com";

    // Act
    let response: Response = test_app.post_subscriptions(body.into()).await;
    dbg!(&response);

    // Assert
    assert_eq!(200, response.status().as_u16());

    // We add now the response!
    // The query! macro verifies the returned struct is valid at run time.
    // it returns an anonymous record type and needs the DATABASE_URL \
    // to verify with, which must be supplied in the `.env` file.
    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@example.com");
    assert_eq!(saved.name, "le guin");
}

/// You need good error messages with parameterised tests to know where assertion failed.
#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    // Arrange
    let test_app: TestApp = spawn_app().await;
    // Table-Driven test aka Parameterised test
    let test_cases: Vec<(&str, &str)> = vec![
        ("name=le%20guin", "Missing the email"),
        ("email=ursula_le_guin%40gmail.com", "Missing the name."),
        ("", "Missing both name and email."),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = test_app.post_subscriptions(invalid_body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // additional customised error message
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

/// Troublesome Inputs
#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app: TestApp = spawn_app().await;
    let test_cases: Vec<(&str, &str)> = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response: Response = app.post_subscriptions(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}",
            description
        );
    }
}
