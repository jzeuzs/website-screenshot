use std::ops::Not;

use actix_web::http::header;
use actix_web::{get, web, HttpResponse};

use crate::error::Error;
use crate::providers::Provider;
use crate::{Result, State};

#[get("/s/{slug}")]
pub async fn get_screenshot(
    data: web::Data<State>,
    slug: web::Path<String>,
) -> Result<HttpResponse, Error> {
    if data.storage.check(slug.clone()).await.expect("Failed checking slug").not() {
        return Err(Error::ScreenshotNotFound);
    }

    // Safe to unwrap now
    let screenshot = data.storage.get(slug.into_inner()).await.unwrap();

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .append_header(header::CacheControl(vec![header::CacheDirective::MaxAge(31_536_000)]))
        .body(screenshot))
}
