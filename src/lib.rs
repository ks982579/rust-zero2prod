use actix_web::{dev::Server, web, App, HttpRequest, HttpResponse, HttpServer};
use std::net::TcpListener;

/// We don't have to pass in `_req` surprisingly.
async fn health_check(_req: HttpRequest) -> HttpResponse {
    // HttpResponse is OK because Responder converts to it anyway
    // HttpResponse::Ok gives us a builder with default 200 status code
    // You could use `.finish()` to build, but the builder itself implements the Responder trait
    HttpResponse::Ok().finish()
}

#[derive(serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

/// Actix-Web calls Form::from_request() on our arguments.
/// It tries to deserialise the body into FormData.
/// If it succeeds, it invokes our `subscribe()` function and carries on...
/// Else, it automagically returns 400 Bad Request.
async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

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
