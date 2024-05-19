//! tests/api/helpers.rs

use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};
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
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

/// Confirmation links embedded in request to the email API.
#[derive(Debug)]
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    /// Fetch Change Password request
    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/password", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }
    /// POST for changing password
    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
    /// Fetching Admin Dashboard response
    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/dashboard", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }
    /// Fetch HTML for Admin page
    pub async fn get_admin_dashboard_html(&self) -> String {
        // self.api_client
        //     .get(&format!("{}/admin/dashboard", &self.address))
        //     .send()
        //     .await
        //     .expect("Failed to execute reqwest.")
        //     // decodes and returns response text (must await)
        //     .text()
        //     .await
        //     .unwrap()
        self.get_admin_dashboard().await.text().await.unwrap()
    }
    /// Tests only look at HTML page, not exposing underlying reqwest::Response
    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute reqwest.")
            // decodes and returns response text (must await)
            .text()
            .await
            .unwrap()
    }
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            // `form()` ensures the body is URL-encoded and `Content-type` header is set
            // accordingly
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
    pub async fn post_subscriptions(&self, body: String) -> Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        dbg!(&body);

        // Extract link from request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link: reqwest::Url = reqwest::Url::parse(&raw_link).unwrap();
            // Ensure not calling random APIs on web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            dbg!(&confirmation_link);
            confirmation_link
        };

        let html: reqwest::Url = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text: reqwest::Url = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        // let (username, password) = self.test_user().await;
        self.api_client
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    // Helper function to retrieve username and password
    // pub async fn test_user(&self) -> (String, String) {
    //     let row = sqlx::query!("SELECT username, password FROM users LIMIT 1",)
    //         .fetch_one(&self.db_pool)
    //         .await
    //         .expect("Failed to create test users.");
    //     (row.username, row.password)
    // }
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        // Don't worry about exact Argon2 parameters in test
        // Now we match parameters of the default password
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        // let password_hash = Argon2::default()
        //     .hash_password(self.password.as_bytes(), &salt)
        //     .unwrap()
        //     .to_string();
        // let password_hash = sha3::Sha3_256::digest(self.password.as_bytes());
        // let password_hash = format!("{:x}", password_hash);
        sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash)
            values ($1, $2, $3)"#,
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
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
    dbg!("Starting Spawn App");
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

    dbg!("Starting database Connection");
    // Create and migrate the database
    configure_database(&configuration.database).await;

    dbg!("Finished database Connection");

    // adding clone of connection pool
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");

    let application_port: u16 = application.port();
    // obtain port before spawing application
    // let address = format!("http://127.0.0.1:{}", application.port());

    let _ = tokio::spawn(application.run_until_stopped());

    dbg!("Here 1?");
    // On Client for All
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    dbg!("Here 2?");

    // Return the String of the whole address.
    // format!("http://127.0.0.1:{}", port)
    // Now, we return a TestApp
    let test_app = TestApp {
        address: format!("http://127.0.0.1:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        test_user: TestUser::generate(),
        api_client: client,
    };
    // add_test_user(&test_app.db_pool).await;
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
}

// async fn add_test_user(pool: &PgPool) {
//     sqlx::query!(
//         r#"
//     INSERT INTO users (user_id, username, password)
//     VALUES ($1, $2, $3)
//     "#,
//         Uuid::new_v4(),
//         Uuid::new_v4().to_string(),
//         Uuid::new_v4().to_string(),
//     )
//     .execute(pool)
//     .await
//     .expect("Failed to create test users.");
// }

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
