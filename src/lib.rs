use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};

/// We don't have to pass in `_req` surprisingly.
async fn health_check(_req: HttpRequest) -> impl Responder {
    // HttpResponse is OK because Responder converts to it anyway
    // HttpResponse::Ok gives us a builder with default 200 status code
    // You could use `.finish()` to build, but the builder itself implements the Responder trait
    HttpResponse::Ok()
}

pub async fn run() -> Result<(), std::io::Error> {
    // HttpServer for binding to TCP socket, maximum number of connections
    // allowing transport layer security, and more.
    HttpServer::new(|| App::new().route("/health-check", web::get().to(health_check)))
        .bind("127.0.0.1:8000")?
        .run()
        .await
}
