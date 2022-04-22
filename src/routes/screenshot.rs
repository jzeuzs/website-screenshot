use actix_web::{post, web, HttpResponse};
use cuid::slug;
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;

use crate::providers::Provider;
use crate::{Result, State};

#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct RequestData {
    #[validate(url)]
    url: String,
}

#[post("/screenshot")]
pub async fn screenshot(
    data: web::Data<State>,
    payload: web::Json<RequestData>,
) -> Result<HttpResponse> {
    let mut client = data.browser.lock().await;

    client.goto(&payload.url).await.expect("Failed navigating to site");
    client.set_window_size(1980, 1080).await.expect("Failed setting window size");
    client
        .execute("document.body.style.overflow = 'hidden'", vec![])
        .await
        .expect("Failed hiding scrollbar");

    let screenshot = client.screenshot().await.expect("Failed screenshoting website");
    let slug = slug().expect("Failed generating slug");

    data.storage.set(slug.clone(), screenshot).await.expect("Failed setting image");

    Ok(HttpResponse::Created().json(json!({
        "slug": &slug,
        "path": format!("/s/{}", &slug),
        "metadata": {
            "url": payload.url
        }
    })))
}
