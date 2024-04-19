// use secrecy::ExposeSecret;
// use std::net::TcpListener;
// use sqlx::{Connection, PgConnection};
// use sqlx::postgres::PgPoolOptions;
// use sqlx::PgPool;

use zero2prod::{
    configuration::{get_configuration, Settings},
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

/// Main function to start service.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Very difficult to pin down concrete type...
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    // Panic if we can't read configuration!
    let configuration: Settings = get_configuration().expect("Failed to read configuration.");

    // Bunch of Logic moved into `startup::build()`
    // Which is now in `Application` struct
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
