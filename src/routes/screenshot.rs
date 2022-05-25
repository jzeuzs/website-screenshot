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

    let req = data.reqwest.get(&payload.url).send().await.map_err(|e| match e {
        err if err.is_redirect() => Error::TooManyRedirects,
        err if err.is_connect() => Error::FailedToConnect,
        _ => Error::WebsiteError,
    })?;

    let url = req.url();
    let check_nsfw = env::var("FORCE_NSFW_CHECK").is_ok() || payload.check_nsfw;
    let dark_mode = env::var("FORCE_DARK_MODE").is_ok() || payload.dark_mode;

    if check_nsfw
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

    if dark_mode {
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
            "check_nsfw": payload.check_nsfw,
            "dark_mode": payload.dark_mode
        }
    })))
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use actix_web::error::ResponseError as ActixResponseError;
    use actix_web::web::Bytes;
    use actix_web::{test, App};
    use serde_json::{from_str, json, Value};

    use super::*;
    use crate::error::Error;
    use crate::middlewares::Auth;
    use crate::util::test::{setup_with_state, ResponseError};

    trait BodyTest {
        fn as_str(&self) -> &str;
        fn as_serde_value(&self) -> Value;
    }

    impl BodyTest for Bytes {
        fn as_str(&self) -> &str {
            unsafe { std::str::from_utf8_unchecked(self) }
        }

        fn as_serde_value(&self) -> Value {
            from_str(self.as_str()).expect("Failed to parse json")
        }
    }

    #[actix_web::test]
    async fn test_screenshot() {
        let (client, state) = setup_with_state().await.expect("Failed setting up test");
        let app = test::init_service(App::new().app_data(state.clone()).service(screenshot)).await;
        let req = test::TestRequest::post()
            .uri("/screenshot")
            .set_json(json!({
                "url": "https://crates.io"
            }))
            .to_request();

        let res = test::call_service(&app, req).await;

        assert_eq!(201, res.status().as_u16());
        assert_eq!(Some("Created"), res.status().canonical_reason());

        let body = to_bytes(res.into_body()).await.unwrap().as_serde_value();

        assert!(body.is_object());

        client.close().await.expect("Failed to close client");
    }

    #[actix_web::test]
    async fn test_screenshot_invalid_url() {
        let (client, state) = setup_with_state().await.expect("Failed setting up test");
        let app = test::init_service(App::new().app_data(state.clone()).service(screenshot)).await;
        let req = test::TestRequest::post()
            .uri("/screenshot")
            .set_json(json!({
                "url": "i am an invalid url"
            }))
            .to_request();

        let res = test::call_service(&app, req).await;

        assert_eq!(400, res.status().as_u16());
        assert_eq!(Some("Bad Request"), res.status().canonical_reason());
        assert_eq!(
            ResponseError(&Error::InvalidUrl as &dyn ActixResponseError),
            ResponseError(
                res.response().error().expect("Failed extracting error").as_response_error()
            )
        );

        assert_eq!(
            "The url that you provided was invalid.",
            to_bytes(res.into_body()).await.unwrap().as_str()
        );

        client.close().await.expect("Failed to close client");
    }

    #[actix_web::test]
    async fn test_screenshot_nsfw_url() {
        let (client, state) = setup_with_state().await.expect("Failed setting up test");
        let app = test::init_service(App::new().app_data(state.clone()).service(screenshot)).await;
        let req = test::TestRequest::post()
            .uri("/screenshot")
            .set_json(json!({
                "url": "https://nhentai.net",
                "check_nsfw": true
            }))
            .to_request();

        let res = test::call_service(&app, req).await;

        assert_eq!(403, res.status().as_u16());
        assert_eq!(Some("Forbidden"), res.status().canonical_reason());
        assert_eq!(
            ResponseError(&Error::UrlNotSafeForWork as &dyn ActixResponseError),
            ResponseError(
                res.response().error().expect("Failed extracting error").as_response_error()
            )
        );

        assert_eq!(
            "The url provided is marked as NSFW.",
            to_bytes(res.into_body()).await.unwrap().as_str()
        );

        client.close().await.expect("Failed to close client");
    }

    #[actix_web::test]
    async fn test_screenshot_auth() {
        std::env::set_var("AUTH_TOKEN", "very_secure_token");

        let (client, state) = setup_with_state().await.expect("Failed setting up test");
        let app =
            test::init_service(App::new().wrap(Auth).app_data(state.clone()).service(screenshot))
                .await;
        let req = test::TestRequest::post()
            .uri("/screenshot")
            .append_header(("Authorization", "very_secure_token"))
            .set_json(json!({
                "url": "https://crates.io"
            }))
            .to_request();

        let res = test::call_service(&app, req).await;

        assert_eq!(201, res.status().as_u16());
        assert_eq!(Some("Created"), res.status().canonical_reason());

        let body = to_bytes(res.into_body()).await.unwrap().as_serde_value();

        assert!(body.is_object());

        client.close().await.expect("Failed to close client");
    }

    #[actix_web::test]
    async fn test_screenshot_auth_missing() {
        std::env::set_var("AUTH_TOKEN", "very_secure_token");

        let (client, state) = setup_with_state().await.expect("Failed setting up test");
        let app =
            test::init_service(App::new().wrap(Auth).app_data(state.clone()).service(screenshot))
                .await;
        let req = test::TestRequest::post()
            .uri("/screenshot")
            .set_json(json!({
                "url": "https://crates.io"
            }))
            .to_request();

        let res = test::call_service(&app, req).await;

        assert_eq!(400, res.status().as_u16());
        assert_eq!(Some("Bad Request"), res.status().canonical_reason());
        assert_eq!(
            ResponseError(&Error::MissingAuthToken as &dyn ActixResponseError),
            ResponseError(
                res.response().error().expect("Failed extracting error").as_response_error()
            )
        );

        assert_eq!(
            "Authentication was enabled but the \"Authorization\" header was not present.",
            to_bytes(res.into_body()).await.unwrap().as_str()
        );

        client.close().await.expect("Failed to close client");
    }

    #[actix_web::test]
    async fn test_screenshot_auth_invalid() {
        std::env::set_var("AUTH_TOKEN", "very_secure_token");

        let (client, state) = setup_with_state().await.expect("Failed setting up test");
        let app =
            test::init_service(App::new().wrap(Auth).app_data(state.clone()).service(screenshot))
                .await;
        let req = test::TestRequest::post()
            .uri("/screenshot")
            .append_header(("Authorization", "not_a_valid_token"))
            .set_json(json!({
                "url": "https://crates.io"
            }))
            .to_request();

        let res = test::call_service(&app, req).await;

        assert_eq!(401, res.status().as_u16());
        assert_eq!(Some("Unauthorized"), res.status().canonical_reason());
        assert_eq!(
            ResponseError(&Error::Unauthorized as &dyn ActixResponseError),
            ResponseError(
                res.response().error().expect("Failed extracting error").as_response_error()
            )
        );

        assert_eq!("Invalid token provided.", to_bytes(res.into_body()).await.unwrap().as_str());

        client.close().await.expect("Failed to close client");
    }
}
