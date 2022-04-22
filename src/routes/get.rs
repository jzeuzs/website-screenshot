use actix_web::http::header;
use actix_web::{get, web, HttpResponse};
use serde_json::json;

use crate::providers::Provider;
use crate::{Result, State};

#[get("/s/{slug}")]
pub async fn get(data: web::Data<State>, slug: web::Path<String>) -> Result<HttpResponse> {
    let screenshot = data.storage.get(slug.into_inner()).await;

    match screenshot {
        Ok(screenshot) => Ok(HttpResponse::Ok()
            .content_type("image/png")
            .append_header(header::CacheControl(vec![header::CacheDirective::MaxAge(31536000)]))
            .body(screenshot)),
        Err(_) => Ok(HttpResponse::NotFound().json(json!({
            "error": 404,
            "message": "The screenshot could not be found."
        }))),
    }
}
