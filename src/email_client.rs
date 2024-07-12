use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl<'a> SendEmailRequest<'a> {
    fn params(self) -> HashMap<String, &'a str> {
        let mut params: HashMap<String, &str> = HashMap::new();
        params.insert(String::from("From"), self.from);
        params.insert(String::from("To"), self.to);
        params.insert(String::from("Subject"), self.subject);
        params.insert(String::from("HtmlBody"), self.html_body);
        params.insert(String::from("TextBody"), self.text_body);
        params
    }
}

pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout_sec: u64,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_sec))
            .build()
            .unwrap();
        Self {
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
        let url = format!("{}/email", self.base_url);
        // let request_body = multipart::Form::new()
        //     .text("from", self.sender.as_ref().to_owned())
        //     .text("to", recipient.as_ref().to_owned())
        //     .text("subject", subject.to_owned())
        //     .text("text", html_content.to_owned());
        // .text("text", text_content.to_owned());

        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        let _builder = self
            .http_client
            .post(url)
            .basic_auth("api", Some(self.authorization_token.expose_secret()))
            // .json(&request_body);
            .form(&request_body.params())
            // .multipart(request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    const TIMEOUT_SEC: u64 = 10;
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use std::collections::HashMap;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailFormMatcher;
    impl wiremock::Match for SendEmailFormMatcher {
        fn matches(&self, request: &Request) -> bool {
            // let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            let result = serde_html_form::from_bytes::<HashMap<String, String>>(&request.body);
            // let result: Result<serde_json::Value, _> = serde_html_form::from_bytes(&request.body);

            if let Ok(body) = result {
                // Check that all the mandatory fields are populated
                // without inspecting the field values
                body.contains_key("From")
                    && body.contains_key("To")
                    && body.contains_key("Subject")
                    && body.contains_key("HtmlBody")
                    && body.contains_key("TextBody")
            } else {
                // If parsing failed, do not match the request
                false
            }
        }
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake()),
            TIMEOUT_SEC,
        );

        Mock::given(header_exists("authorization"))
            .and(header("Content-Type", "application/x-www-form-urlencoded"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailFormMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();

        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        // Act
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake()),
            TIMEOUT_SEC,
        );
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        Mock::given(any())
            // Not a 200 anymore!
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;
        // Act
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(Faker.fake()),
            TIMEOUT_SEC,
        );
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        let response = ResponseTemplate::new(200)
            // 3 minutes!
            .set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;
        // Act
        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
        // Assert
        assert_err!(outcome);
    }
}
