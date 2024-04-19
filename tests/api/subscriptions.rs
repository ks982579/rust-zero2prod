//! tests/api/subscriptions.rs

use crate::helpers::{spawn_app, TestApp};
use reqwest::{Client, Response};
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
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

    // To get test passing, must send email as well.
    // This is the pretent PostMark endpoint, returning 200 OK
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // book does not include this, probably because test isn't meant for email
        // .expect(1)
        .mount(&test_app.email_server)
        .await;

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

#[tokio::test]
async fn subscibe_persists_the_new_subscriber() {
    // Arrange
    let test_app: TestApp = spawn_app().await;
    let body: &str = "name=le%20guin&email=ursula_le_guin%40example.com";

    // To get test passing, must send email as well.
    // This is the pretent PostMark endpoint, returning 200 OK
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // book does not include this, probably because test isn't meant for email
        // .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act
    let response: Response = test_app.post_subscriptions(body.into()).await;
    dbg!(&response);

    // Assert
    /*
     * The test above checks the response.
     * This test is for checking the data is in the database.
     * */
    // We add now the response!
    // The query! macro verifies the returned struct is valid at run time.
    // it returns an anonymous record type and needs the DATABASE_URL \
    // to verify with, which must be supplied in the `.env` file.
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@example.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
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

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    // Mock asserts on drop
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // No exception because focus on checking for link
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    // Get first intercepted request
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    // Parse body as JSON, use `from_slice()` because raw bytes
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    // Extract LINK
    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };
    let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(&body["TextBody"].as_str().unwrap());

    // Links should be identical...
    assert_eq!(html_link, text_link);
}
