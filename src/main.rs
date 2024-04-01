use std::net::TcpListener;

use zero2prod::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener: TcpListener = TcpListener::bind("127.0.0.1:8080")?;
    run(listener)?.await
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     async fn health_check_succeeds() {
//         let response = health_check().await;
//         assert!(response.status().is_success());
//     }
// }
