use actix_web::{dev::Server, middleware::Logger, web, App, HttpServer};
// use sqlx::PgConnection;
use sqlx::PgPool;
use std::net::TcpListener;

use crate::routes::{health_check, subscribe};

// removed the `async` from this function.
// adding the `address: &str` parameter to allow for dynamic connections
// Just kidding, we need a TcpListener so we can track the port.
pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    // Wrap the connection in Smart Pointer!
    let db_pool: web::Data<PgPool> = web::Data::new(db_pool);
    // HttpServer for binding to TCP socket, maximum number of connections
    // allowing transport layer security, and more.
    let server: Server = HttpServer::new(move || {
        App::new()
            // middleware added with the `.wrap()` method.
            .wrap(Logger::default())
            .route("/health-check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // Register connection as part of application state
            // connection must be cloneable for every copy of App returned...
            .app_data(db_pool.clone())
    })
    //.bind("127.0.0.1:8000")?
    // .bind(address)? // just kidding, we need to listen, not bind
    .listen(listener)?
    .run();
    // Removed the `.await` here.
    Ok(server)
}
