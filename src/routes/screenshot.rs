use std::env;

use actix_web::{post, web, HttpResponse};
use cuid::slug;
use fantoccini::Locator;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::cdp::ChromeCommand;
use crate::error::Error;
use crate::providers::Provider;
use crate::util::{check_if_nsfw, check_if_url};
use crate::{Result, State};

#[inline]
fn default_fullscreen() -> bool {
    env::var("FULLSCREEN_SCREENSHOT").is_ok()
}

#[inline]
fn default_check_nsfw() -> bool {
    env::var("CHECK_IF_NSFW").is_ok()
}

#[inline]
fn default_dark_mode() -> bool {
    env::var("DARK_MODE").is_ok()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestData {
    url: String,
    #[serde(default = "default_fullscreen")]
    fullscreen: bool,
    #[serde(default = "default_check_nsfw")]
    check_nsfw: bool,
    #[serde(default = "default_dark_mode")]
    dark_mode: bool,
}

#[post("/screenshot")]
pub async fn screenshot(
    data: web::Data<State>,
    payload: web::Json<RequestData>,
) -> Result<HttpResponse, Error> {
    check_if_url(&payload.url).map_err(|_| Error::InvalidUrl)?;

    let req =
        data.reqwest.head(&payload.url).send().await.expect("Failed sending head request to url");
    let url = req.url();

    if payload.check_nsfw
        && check_if_nsfw(url.host_str().expect("Failed getting url host"))
            .await
            .expect("Failed checking if nsfw")
    {
        return Err(Error::UrlNotSafeForWork);
    }

    let client = &data.browser;

    client.goto(url.as_str()).await.expect("Failed navigating to site");
    client.set_window_size(1980, 1080).await.expect("Failed setting window size");
    client
        .execute(
            "\
            document.body.style.overflowX = 'hidden';
            document.body.style.overflowY = 'hidden';
        ",
            vec![],
        )
        .await
        .expect("Failed hiding scrollbar");

    if payload.dark_mode {
        client
            .issue_cmd(ChromeCommand::ExecuteCdpCommand(
                "Emulation.setEmulatedMedia".to_owned(),
                json!({
                    "features": [
                        {
                            "name": "prefers-color-scheme",
                            "value": "dark"
                        }
                    ]
                }),
            ))
            .await
            .expect("Failed issuing cmd");
    } else {
        client
            .issue_cmd(ChromeCommand::ExecuteCdpCommand(
                "Emulation.setEmulatedMedia".to_owned(),
                json!({
                    "features": [
                        {
                            "name": "prefers-color-scheme",
                            "value": "light"
                        }
                    ]
                }),
            ))
            .await
            .expect("Failed issuing cmd");
    }

    client.refresh().await.expect("Failed to refresh");

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
            "url": &payload.url,
            "fullscreen": payload.fullscreen,
            "check_nsfw": payload.check_nsfw
        }
    })))
}
