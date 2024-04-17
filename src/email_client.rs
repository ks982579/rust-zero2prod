//! src/email_client.rs
use crate::domain::SubscriberEmail;
use reqwest::{Client, Response};
use secrecy::{ExposeSecret, Secret};

#[derive(Debug)]
pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    // So to not accidently log
    authorization_token: Secret<String>,
}

#[derive(Debug, serde::Serialize)]
struct SendEmailRequest {
    from: String,
    to: String,
    subject: String,
    html_body: String,
    text_body: String,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
    ) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
            sender,
            authorization_token,
        }
    }
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        // todo!();
        let url: String = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref().to_owned(),
            to: recipient.as_ref().to_owned(),
            subject: subject.to_owned(),
            html_body: html_content.to_owned(),
            text_body: text_content.to_owned(),
        };
        // we get a _builder_... that we turn into Response
        let builder: Response = self
            .http_client
            .post(&url)
            // There is also `.headers()` which takes in a HashMap
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            // `.json()` will serialize AND set `Content-Type: application/json` header.
            .json(&request_body)
            .send()
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{domain::SubscriberEmail, email_client::EmailClient};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use secrecy::Secret;
    // use wiremock::matchers::any;
    use wiremock::{
        matchers::{header, header_exists, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn send_email_fires_request_to_base_url() {
        // Arrange
        // `MockServer` is HTTP server
        let mock_server: MockServer = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        // `.uri()` method gets address of mock server.
        let email_client = EmailClient::new(mock_server.uri(), sender, Secret::new(Faker.fake()));

        // `MockServer` returns 404 to all requests by default.
        // Think of this like setting up our MockServer with new configuration.
        Mock::given(header_exists("X-Postmark-Server-Token"))
            // Chaining matchers together with `.and()`
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            // (1..) for at least one request, or (1..=3) for 1 to 3 requests...
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        // Using random data implies we are not testing content, so please ignore, basically
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        // Act
        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        // Assert
        // Expectations are verified when `MockServer` goes out of scope.
    }
}
