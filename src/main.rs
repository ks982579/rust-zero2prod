use std::net::TcpListener;

use zero2prod::startup::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener: TcpListener = TcpListener::bind("127.0.0.1:8080")?;
    run(listener)?.await
}
