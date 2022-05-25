use std::ops::Deref;

use actix_web::error::ResponseError;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use derive_more::{Display, Error as DeriveError};

#[derive(Debug, Display, DeriveError, PartialEq, Eq)]
pub enum Error {
    #[display(fmt = "The url that you provided was invalid.")]
    InvalidUrl,
    #[display(
        fmt = "Authentication was enabled but the \"Authorization\" header was not present."
    )]
    MissingAuthToken,
    #[display(fmt = "Invalid token provided.")]
    Unauthorized,
    #[display(fmt = "The screenshot with that slug can't be found.")]
    ScreenshotNotFound,
    #[display(fmt = "The url provided is marked as NSFW.")]
    UrlNotSafeForWork,
    #[display(fmt = "The url provided has too many redirects.")]
    TooManyRedirects,
    #[display(fmt = "Failed to connect to the url provided.")]
    FailedToConnect,
    #[display(fmt = "An error occured when accessing the website.")]
    WebsiteError,
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::plaintext())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self.deref() {
            Error::InvalidUrl
            | Error::MissingAuthToken
            | Error::FailedToConnect
            | Error::WebsiteError => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::ScreenshotNotFound => StatusCode::NOT_FOUND,
            Error::UrlNotSafeForWork => StatusCode::FORBIDDEN,
            Error::TooManyRedirects => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }
}
