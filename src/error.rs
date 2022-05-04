use std::ops::Deref;

use actix_web::error::ResponseError;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use derive_more::{Display, Error as DeriveError};

#[derive(Debug, Display, DeriveError)]
pub enum Error {
    #[display(fmt = "The url that you provided was invalid.")]
    InvalidUrl,
    #[display(
        fmt = "Authentication was enabled but the \"Authorization\" header was not present."
    )]
    MissingAuthToken,
    #[display(fmt = "Invalid token provided.")]
    Unauthorized,
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::plaintext())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self.deref() {
            Error::InvalidUrl | Error::MissingAuthToken => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }
}
