/**
* `tokio::test` is like `tokio::main`
* I tried going without and you can't runs tests async.
* Use `cargo expand --test health_check` (name of file) if you are curious
**/

// Launch application somehow in background
// This is only piece coupled with our application.
async fn spawn_app() -> Result<(), std::io::Error> {
    todo!();
}

#[tokio::test]
async fn health_check_success() {
    // Arrange
    spawn_app().await.expect("Failed to spawn app.");
    // bring in `reqwest` to send HTTP requests against application.
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get("http://127.0.0.1:8000/health-check")
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
