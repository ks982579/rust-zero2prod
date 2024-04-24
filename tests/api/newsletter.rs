//! tests/api/newsletter.rs
use crate::helpers::*;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

/// Using public API of application under test to create
/// an unconfirmed subscriber.
async fn create_unconfirmed_subscriber(app: &TestApp) {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create Unconfirmed Subscriber")
        .expect(1)
        // for only mounting in scope, does not persist throughout test
        .mount_as_scoped(&app.email_server)
        .await;

    // If it calls the EmailClient, that will send the confirmation email
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request should be sent
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    // Sketch of newsletter payload
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}
