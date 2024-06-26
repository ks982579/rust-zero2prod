use crate::authentication::reject_anonymous_users;
use actix_web::{cookie::Key, dev::Server, web, App, HttpServer};
use actix_web_lab::middleware::from_fn;
// use sqlx::PgConnection;
use crate::routes::log_out;
use crate::routes::{change_password, change_password_form};
use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::{ExposeSecret, Secret};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::{
    admin_dashboard, confirm, health_check, home, login, login_form, publish_newletter, subscribe,
};

// Need a type to hold serve and its port
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    // Convert `build()` into constructor for `Application`
    // Extracting from main for testing purposes.
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        // // Connect to Database in Main Function!
        // let connection: PgConnection =
        //     PgConnection::connect(&configuration.database.connection_string())
        //         .await
        //         .expect("Failed to connect to Postgres.");
        // let connection_pool: PgPool = PgPool::connect_lazy(&configuration.database.with_db())
        //     .expect("Failed to connect to Postgres.");
        /*
        let connection_pool: PgPool =
            PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());
        */
        let connection_pool: PgPool = get_connection_pool(&configuration.database);

        // Building `EmailClient` using `configuration`
        let sender_email: SubscriberEmail = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let timeout = configuration.email_client.timeout();
        let email_client: EmailClient = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        // Update port based on new settings
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let listener: TcpListener = TcpListener::bind(address)?;
        let port: u16 = listener.local_addr().unwrap().port();
        let server: Server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
        )
        .await?;

        // And we store information in Application struct
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // Expressive name to describe it only returns when application stopped.
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

// We need a wrapper to retrieve URL from `subscribe` handler.
// Retrieval from context in actix-web is type-based
// using raw `String` exposes us to conflicts - right...
pub struct ApplicationBaseUrl(pub String);

// removed the `async` from this function.
// adding the `address: &str` parameter to allow for dynamic connections
// Just kidding, we need a TcpListener so we can track the port.
// And now Asynchronous for Redis
pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    // Wrap the connection in Smart Pointer!
    let db_pool: web::Data<PgPool> = web::Data::new(db_pool);
    let email_client: web::Data<EmailClient> = web::Data::new(email_client);
    let base_url: web::Data<ApplicationBaseUrl> = web::Data::new(ApplicationBaseUrl(base_url));

    // This is storage backend
    // It requres a key to sign cookies
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    // Requires storage backend as argument...
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    // HttpServer for binding to TCP socket, maximum number of connections
    // allowing transport layer security, and more.
    let server: Server = HttpServer::new(move || {
        App::new()
            // middleware added with the `.wrap()` method.
            // .wrap(Logger::default())
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/health-check", web::get().to(health_check))
            .route("/newsletters", web::post().to(publish_newletter))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .service(
                web::scope("/admin")
                    // We can insert endpoint specific middleware here
                    .wrap(from_fn(reject_anonymous_users))
                    .route("/dashboard", web::get().to(admin_dashboard))
                    .route("/logout", web::post().to(log_out))
                    .route("/password", web::get().to(change_password_form))
                    .route("/password", web::post().to(change_password)),
            )
            // Register connection as part of application state
            // connection must be cloneable for every copy of App returned...
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(web::Data::new(HmacSecret(hmac_secret.clone())))
    })
    //.bind("127.0.0.1:8000")?
    // .bind(address)? // just kidding, we need to listen, not bind
    .listen(listener)?
    .run();
    // Removed the `.await` here.
    Ok(server)
}

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);
