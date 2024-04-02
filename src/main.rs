use std::net::TcpListener;

use sqlx::{Connection, PgConnection};
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Panic if we can't read configuration!
    let configuration: Settings = get_configuration().expect("Failed to read configuration.");
    // Connect to Database in Main Function!
    let connection: PgConnection =
        PgConnection::connect(&configuration.database.connection_string())
            .await
            .expect("Failed to connect to Postgres.");
    // Update port based on new settings
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener: TcpListener = TcpListener::bind(address)?;
    run(listener, connection)?.await
}
