use std::net::TcpListener;

// use env_logger::Env;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{
    layer::{Layered, SubscriberExt},
    EnvFilter, Registry,
};
// use sqlx::{Connection, PgConnection};
use sqlx::PgPool;

use zero2prod::{
    configuration::{get_configuration, Settings},
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

/// Main function to start service.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Very difficult to pin down concrete type...
    let subscriber = get_subscriber("zero2prod".into(), "info".into());
    init_subscriber(subscriber);
    // Panic if we can't read configuration!
    let configuration: Settings = get_configuration().expect("Failed to read configuration.");
    // // Connect to Database in Main Function!
    // let connection: PgConnection =
    //     PgConnection::connect(&configuration.database.connection_string())
    //         .await
    //         .expect("Failed to connect to Postgres.");
    let connection_pool: PgPool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // Update port based on new settings
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener: TcpListener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}
