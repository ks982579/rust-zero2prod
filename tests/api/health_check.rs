//! tests/api/health_check.rs

use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn health_check_success() {
    // Arrange
    // No .await or .expect required now...
    //spawn_app().await.expect("Failed to spawn app.");
    let test_app: TestApp = spawn_app().await;
    // bring in `reqwest` to send HTTP requests against application.
    let client = reqwest::Client::new();

    // Act
    let response = client
        //.get("http://127.0.0.1:8000/health-check")
        .get(&format!("{}/health-check", &test_app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
