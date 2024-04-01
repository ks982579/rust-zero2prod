use std::net::TcpListener;

/**
* `tokio::test` is like `tokio::main`
* I tried going without and you can't runs tests async.
* Use `cargo expand --test health_check` (name of file) if you are curious
**/

// Launch application somehow in background
// This is only piece coupled with our application.
// Remove the `async` here as well...
fn spawn_app() -> String {
    let listener: TcpListener =
        TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port.");
    // Retrieve the port assigned by the OS
    let port: u16 = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    // Return the String of the whole address.
    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_success() {
    // Arrange
    // No .await or .expect required now...
    //spawn_app().await.expect("Failed to spawn app.");
    let address: String = spawn_app();
    // bring in `reqwest` to send HTTP requests against application.
    let client = reqwest::Client::new();

    // Act
    let response = client
        //.get("http://127.0.0.1:8000/health-check")
        .get(&format!("{}/health-check", &address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let app_address: String = spawn_app();
    let client: reqwest::Client = reqwest::Client::new();

    // Act
    let body: &str = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());
}
