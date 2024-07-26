use actix_web::{
    error::InternalError,
    http::{header::LOCATION, StatusCode},
    web, HttpResponse, ResponseError,
};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            // let query_string = format!("error={}", urlencoding::Encoded::new(e.to_string()));
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .insert_header(("Set-Cookie", format!("_flash={e}")))
                .finish();

            Err(InternalError::from_response(e, response))
        }
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl ResponseError for LoginError {
    // fn error_response(&self) -> HttpResponse {
    //     let query_string = format!("error={}", urlencoding::Encoded::new(self.to_string()));
    //     let secret: &[u8] = todo!();
    //     let hmac_tag = {
    //         let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
    //         mac.update(query_string.as_bytes());
    //         mac.finalize().into_bytes()
    //     };

    //     HttpResponse::build(self.status_code())
    //         // Appending the hexadecimal representation of the HMAC tag to the
    //         // query string as an additional query parameter.
    //         .insert_header((LOCATION, format!("/login?{query_string}&tag={hmac_tag:x}")))
    //         .finish()
    // }

    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }
}
