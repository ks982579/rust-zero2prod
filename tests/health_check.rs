use reqwest::Response;
use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::startup::run;

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
    let server = run(listener).expect("Failed to bind address");
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
    // We want to connect to the database also
    let configuration: Settings = get_configuration().expect("Failed to read configuration.");
    let connection_string: String = configuration.database.connection_string();
    // Note: `Connection` trait must be in scope to invoke
    // `PgConnection::connect` - it is not an inherent method of the struct!
    // Also, the return type of `.connect()` is wild...
    let mut connection: PgConnection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to Postgres");
    let client: reqwest::Client = reqwest::Client::new();

    // Act
    let body: &str = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response: Response = client
        .post(&format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());

    // We add now the response!
    // The query! macro verifies the returned struct is valid at run time.
    // it returns an anonymous record type and needs the DATABASE_URL \
    // to verify with, which must be supplied in the `.env` file.
    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

/// You need good error messages with parameterised tests to know where assertion failed.
#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    // Arrange
    let app_address: String = spawn_app();
    let client: reqwest::Client = reqwest::Client::new();
    // Table-Driven test aka Parameterised test
    let test_cases: Vec<(&str, &str)> = vec![
        ("name=le%20guin", "Missing the email"),
        ("email=ursula_le_guin%40gmail.com", "Missing the name."),
        ("", "Missing both name and email."),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(&format!("{}/subscriptions", &app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

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
