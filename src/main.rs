use zero2prod::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    run().await
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
