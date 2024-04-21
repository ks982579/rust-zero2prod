//! tests/api/helpers.rs

// use reqwest::{Client, Response};
// use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
//use std::net::TcpListener;
// use tracing_subscriber::fmt::format;
use reqwest::{Client, Response};
use std::sync::OnceLock;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings, Settings},
    // domain::SubscriberEmail,
    // email_client::EmailClient,
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

/**
* `tokio::test` is like `tokio::main`
* I tried going without and you can't runs tests async.
* Use `cargo expand --test health_check` (name of file) if you are curious.
* Also, most tests started out in here for convenience.
* However, given an endpoint, integration tests should have corresponding folders.
**/

// A synchronization primitive that can only be written to once (allows for one trace).
static TRACING: OnceLock<()> = OnceLock::new();

/// Struct to hold app connection information.
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
}

/// Confirmation links embedded in request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> Response {
        Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract link from request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Ensure not calling random APIs on web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html: reqwest::Url = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text: reqwest::Url = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }
}

/// To create test DB and run migrations
async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create Database
    let mut connection: PgConnection =
        // PgConnection::connect(&config.connection_string_without_db().expose_secret())
        PgConnection::connect_with(&config.without_db())
             .await
             .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    // let connection_pool = PgPool::connect(&config.connection_string().expose_secret())
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

// Launch application somehow in background
// This is only piece coupled with our application.
// Remove the `async` here as well...
// Making function async now
pub async fn spawn_app() -> TestApp {
    // Subscribing to Trace events (like in `main()`)
    TRACING.get_or_init(|| {
        if std::env::var("TEST_LOG").is_ok() {
            let subscriber =
                get_subscriber("test".to_string(), "info".to_string(), std::io::stdout);
            init_subscriber(subscriber);
        } else {
            let subscriber = get_subscriber("test".to_string(), "info".to_string(), std::io::sink);
            init_subscriber(subscriber);
        }
    });

    // Launch mock server to stand in for Postmark's API
    let email_server: MockServer = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let mut configuration: Settings = get_configuration().expect("Failed to read configuration.");
    // use a different database for each test
    configuration.database.database_name = Uuid::new_v4().to_string();
    // Use a random OS port
    configuration.application.port = 0;
    // Use mock server as email API
    configuration.email_client.base_url = email_server.uri();

    // Create and migrate the database
    configure_database(&configuration.database).await;

    // adding clone of connection pool
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");

    let application_port: u16 = application.port();
    // obtain port before spawing application
    // let address = format!("http://127.0.0.1:{}", application.port());

    let _ = tokio::spawn(application.run_until_stopped());
    // Return the String of the whole address.
    // format!("http://127.0.0.1:{}", port)
    // Now, we return a TestApp
    TestApp {
        address: format!("http://127.0.0.1:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
    }
}
