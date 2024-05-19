//! tests/api/login.rs
use std::collections::HashSet;

use crate::helpers::assert_is_redirect_to;
use crate::helpers::spawn_app;
use reqwest::header::HeaderValue;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = app.post_login(&login_body).await;

    // Assert
    // assert_eq!(response.status().as_u16(), 303);
    assert_is_redirect_to(&response, "/login");

    // let cookies: HashSet<_> = response
    //     .headers()
    //     .get_all("Set-Cookie")
    //     .into_iter()
    //     .collect();
    // assert!(cookies.contains(&HeaderValue::from_str("_flash=Authentication Failed").unwrap()));
    let flash_cookie: reqwest::cookie::Cookie =
        response.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(flash_cookie.value(), "Authentication Failed");

    // Act - Part 2 - Follow Redirect
    let html_page: String = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication Failed</i></p>"#));

    // Act - Part 3 - Reload
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(r#"Authentication Failed"#));
}
