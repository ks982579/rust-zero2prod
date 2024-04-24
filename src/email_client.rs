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
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        // extract builder here to set timeout.
        let http_client: Client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            // http_client: Client::new(),
            http_client,
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
            from: self.sender.as_ref().as_ref(),
            to: recipient.as_ref().as_ref(),
            subject: subject.as_ref(),
            html_body: html_content.as_ref(),
            text_body: text_content.as_ref(),
        };
        // we get a _builder_... that we turn into Response
        let _builder: Response = self
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
            .await?
            // If the server gets a response, it is OK.
            // We need to tell it some responses should be errors.
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{domain::SubscriberEmail, email_client::EmailClient};
    use claims::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use secrecy::Secret;
    // use validator::ValidateRequired;
    use wiremock::{
        matchers::{any, header, header_exists, method, path},
        Mock, MockServer, Request, ResponseTemplate,
    };

    // Making test specific tools
    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            //unimplemented!();
            // Try parse body as JSON
            // `from_slice()` parses from bytes, which is what HTTP request is
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                // Check fields are populated w/out checking value
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                // Fails if not maching
                false
            }
        }
    }

    /// Generate a random email subject
    fn subject() -> String {
        // Using random data implies we are not testing content, so please ignore, basically
        Sentence(1..2).fake()
    }

    /// Generate a random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Generate a radom subscriber email
    fn email() -> SubscriberEmail {
        // * Think of `SafeEmail()` as a builder, and `.fake()` as the build step
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    /// Get a test instancec of `EmailClient`
    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // Arrange
        // `MockServer` is HTTP server
        let mock_server: MockServer = MockServer::start().await;

        // `.uri()` method gets address of mock server.
        let email_client = email_client(mock_server.uri());

        // `MockServer` returns 404 to all requests by default.
        // Think of this like setting up our MockServer with new configuration.
        Mock::given(header_exists("X-Postmark-Server-Token"))
            // Chaining matchers together with `.and()`
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            // Insert Custom Matcher!
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            // (1..) for at least one request, or (1..=3) for 1 to 3 requests...
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        // Expectations are verified when `MockServer` goes out of scope.
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // Do not copy in all matchers, those are tested previously...
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_eamil_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response: ResponseTemplate =
            ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }
}
