use actix_web::{dev::Server, web, App, HttpServer};
use std::net::TcpListener;

use crate::routes::{health_check, subscribe};

// removed the `async` from this function.
// adding the `address: &str` parameter to allow for dynamic connections
// Just kidding, we need a TcpListener so we can track the port.
pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    // HttpServer for binding to TCP socket, maximum number of connections
    // allowing transport layer security, and more.
    let server: Server = HttpServer::new(|| {
        App::new()
            .route("/health-check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
    })
    //.bind("127.0.0.1:8000")?
    // .bind(address)? // just kidding, we need to listen, not bind
    .listen(listener)?
    .run();
    // Removed the `.await` here.
    Ok(server)
}
