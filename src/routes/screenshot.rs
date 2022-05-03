use std::env;

use actix_web::{post, web, HttpResponse};
use cuid::slug;
use fantoccini::Locator;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::Error;
use crate::providers::Provider;
use crate::util::check_if_url;
use crate::{Result, State};

#[inline]
fn default_fullscreen() -> bool {
    env::var("FULLSCREEN_SCREENSHOT").is_ok()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestData {
    url: String,
    #[serde(default = "default_fullscreen")]
    fullscreen: bool,
}

#[post("/screenshot")]
pub async fn screenshot(
    data: web::Data<State>,
    payload: web::Json<RequestData>,
) -> Result<HttpResponse, Error> {
    check_if_url(&payload.url).map_err(|_| Error::InvalidUrl)?;

    let client = &data.browser;

    client.goto(&payload.url).await.expect("Failed navigating to site");
    client.set_window_size(1980, 1080).await.expect("Failed setting window size");
    client
        .execute("document.body.style.overflow = 'hidden'", vec![])
        .await
        .expect("Failed hiding scrollbar");

    let screenshot = match payload.fullscreen {
        true => {
            let original_size = client.get_window_size().await.expect("Failed to get window size");
            let width = client
                .execute("return document.body.parentNode.scrollWidth", vec![])
                .await
                .expect("Failed getting scroll width")
                .as_u64()
                .expect("Failed to convert to u64");

            let height = client
                .execute("return document.body.parentNode.scrollHeight", vec![])
                .await
                .expect("Failed getting scroll height")
                .as_u64()
                .expect("Failed to convert to u64");

            client
                .set_window_size(width as u32, height as u32)
                .await
                .expect("Failed setting window size");

            let ss = client
                .find(Locator::Css("body"))
                .await
                .expect("Failed finding body element")
                .screenshot()
                .await
                .expect("Failed screenshoting page");

            client
                .set_window_size(original_size.0 as u32, original_size.1 as u32)
                .await
                .expect("Failed setting window size");

            ss
        },
        false => client.screenshot().await.expect("Failed screenshoting page"),
    };

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
