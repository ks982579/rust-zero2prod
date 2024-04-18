use actix_web::{dev::Server, web, App, HttpServer};
// use sqlx::PgConnection;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};

// Need a type to hold serve and its port
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    // Convert `build()` into constructor for `Application`
    // Extracting from main for testing purposes.
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
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
        let server: Server = run(listener, connection_pool, email_client)?;

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

// removed the `async` from this function.
// adding the `address: &str` parameter to allow for dynamic connections
// Just kidding, we need a TcpListener so we can track the port.
pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    // Wrap the connection in Smart Pointer!
    let db_pool: web::Data<PgPool> = web::Data::new(db_pool);
    let email_client: web::Data<EmailClient> = web::Data::new(email_client);
    // HttpServer for binding to TCP socket, maximum number of connections
    // allowing transport layer security, and more.
    let server: Server = HttpServer::new(move || {
        App::new()
            // middleware added with the `.wrap()` method.
            // .wrap(Logger::default())
            .wrap(TracingLogger::default())
            .route("/health-check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // Register connection as part of application state
            // connection must be cloneable for every copy of App returned...
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    //.bind("127.0.0.1:8000")?
    // .bind(address)? // just kidding, we need to listen, not bind
    .listen(listener)?
    .run();
    // Removed the `.await` here.
    Ok(server)
}
