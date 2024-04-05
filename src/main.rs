use std::net::TcpListener;

// use env_logger::Env;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{
    layer::{Layered, SubscriberExt},
    EnvFilter, Registry,
};
// use sqlx::{Connection, PgConnection};
use sqlx::PgPool;

use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Redirecting all log's events to our subscriber...
    LogTracer::init().expect("Failed to set Logger");
    // // `init` calles `set_logger` for us, and we default to "info".
    // env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    // Removed env_logger...
    let env_filter: EnvFilter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer: BunyanFormattingLayer<fn() -> std::io::Stdout> =
        BunyanFormattingLayer::new(
            "zero2prod".into(),
            // output formatted spans to stdout.
            std::io::stdout,
        );
    // `with` is provided by `SubscriberExt`
    let subscriber: Layered<
        BunyanFormattingLayer<fn() -> std::io::Stdout>,
        Layered<JsonStorageLayer, Layered<EnvFilter, Registry>>,
    > = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    // below specifies what subscriber should be used to procecss spans.
    set_global_default(subscriber).expect("Failed to set subscriber.");
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
