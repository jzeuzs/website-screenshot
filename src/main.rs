#![feature(let_chains, pattern)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::unreadable_literal,
    clippy::module_name_repetitions,
    clippy::unused_async,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::wildcard_imports
)]

#[macro_use]
extern crate tracing;

use std::borrow::ToOwned;
use std::env;
use std::sync::Arc;

use actix_web::middleware::Compress;
use actix_web::{web, App, Error, HttpServer};
use actix_web_static_files::ResourceFiles;
use cdp::ChromeCommand;
use evasions::{
    evaluate_on_new_document,
    CHROME_APP,
    CHROME_CSI,
    CHROME_LOADTIMES,
    CHROME_RUNTIME,
    IFRAME_CONTENTWINDOW,
    MEDIA_CODECS,
    NAVIGATOR_HARDWARECONCURRENCY,
    NAVIGATOR_LANGUAGES,
    NAVIGATOR_PERMISSIONS,
    NAVIGATOR_PLUGINS,
    NAVIGATOR_VENDOR,
    NAVIGATOR_WEBDRIVER,
    UTILS,
    WEBGL_VENDOR,
    WINDOW_OUTERDIMENSIONS,
};
use fantoccini::{Client, ClientBuilder};
use providers::{Provider, Storage};
use reqwest::Client as ReqwestClient;
use serde_json::{Map, Value};
use tracing_actix_web::TracingLogger;
use util::{initialize_tracing, load_env};
use website_screenshot_actix_governor::{Governor, GovernorConfigBuilder};

pub mod cdp;
pub mod error;
pub mod evasions;
pub mod middlewares;
pub mod providers;
pub mod routes;
pub mod util;

pub type Result<T, E = Error> = anyhow::Result<T, E>;

#[derive(Debug)]
pub struct State {
    pub browser: Arc<Client>,
    pub storage: Arc<Storage>,
    pub reqwest: ReqwestClient,
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    load_env();
    initialize_tracing();

    let mut capabilities = Map::new();
    let mut args = vec![
        "--disable-gpu".to_owned(),
        "--no-sandbox".to_owned(),
        "--disable-dev-shm-usage".to_owned(),
        "--headless".to_owned(),
        "--hide-scrollbars".to_owned(),
        "--whitelisted-ips=".to_owned(),
    ];

    if let Ok(flags) = env::var("CHROME_FLAGS") {
        let flags = flags.split(',').map(ToOwned::to_owned).collect::<Vec<_>>();

        args.extend_from_slice(&flags);
    };

    let chrome_opts = match env::var("GOOGLE_CHROME_PATH") {
        Ok(path) => serde_json::json!({
            "binary": path,
            "args": args
        }),
        Err(_) => serde_json::json!({ "args": args }),
    };

    capabilities.insert("goog:chromeOptions".to_owned(), chrome_opts);

    let chromedriver_address = match env::var("CHROMEDRIVER_ADDRESS") {
        Ok(address) => address,
        Err(_) => "http://localhost:9515".to_owned(),
    };

    let client =
        ClientBuilder::rustls().capabilities(capabilities).connect(&chromedriver_address).await?;

    info!("Connected to chromedriver at localhost:9515");

    // To hide headless nature (for Cloudflare, etc.)
    tokio::join!(
        evaluate_on_new_document(&client, UTILS, vec![]),
        evaluate_on_new_document(&client, CHROME_APP, vec![]),
        evaluate_on_new_document(&client, CHROME_CSI, vec![]),
        evaluate_on_new_document(&client, CHROME_LOADTIMES, vec![]),
        evaluate_on_new_document(&client, CHROME_RUNTIME, vec![Value::Bool(false)]),
        evaluate_on_new_document(&client, IFRAME_CONTENTWINDOW, vec![]),
        evaluate_on_new_document(&client, MEDIA_CODECS, vec![]),
        evaluate_on_new_document(&client, NAVIGATOR_HARDWARECONCURRENCY, vec![Value::Number(
            4.into()
        )]),
        evaluate_on_new_document(&client, NAVIGATOR_LANGUAGES, vec![Value::Array(vec![
            "en-US".into(),
            "en".into()
        ])]),
        evaluate_on_new_document(&client, NAVIGATOR_PERMISSIONS, vec![]),
        evaluate_on_new_document(&client, NAVIGATOR_PLUGINS, vec![]),
        evaluate_on_new_document(&client, NAVIGATOR_VENDOR, vec![Value::String(
            "Google Inc.".to_owned()
        )]),
        evaluate_on_new_document(&client, NAVIGATOR_WEBDRIVER, vec![]),
        evaluate_on_new_document(&client, WEBGL_VENDOR, vec![
            Value::String("Intel Inc.".to_owned()),
            Value::String("Intel Iris OpenGL Engine".to_owned())
        ]),
        evaluate_on_new_document(&client, WINDOW_OUTERDIMENSIONS, vec![]),
    );

    // Override user-agent
    let user_agent = client
        .issue_cmd(ChromeCommand::ExecuteCdpCommand(
            "Browser.getVersion".to_owned(),
            serde_json::json!({}),
        ))
        .await
        .expect("Failed issuing cmd")["userAgent"]
        .as_str()
        .expect("Failed to get user agent")
        .to_owned();

    let new_user_agent = user_agent.replace("HeadlessChrome", "Chrome");

    client
        .issue_cmd(ChromeCommand::ExecuteCdpCommand(
            "Network.setUserAgentOverride".to_owned(),
            serde_json::json!({
                "userAgent": new_user_agent,
                "acceptLanguage": "en-US,en"
            }),
        ))
        .await
        .expect("Failed issuing cmd");

    let governor_config = GovernorConfigBuilder::default()
        .per_second(60)
        .burst_size(20)
        .with_headers()
        .finish()
        .expect("Failed to build ratelimiter");

    let state = web::Data::new(State {
        browser: Arc::new(client.clone()),
        storage: Arc::new(Storage::new()),
        reqwest: ReqwestClient::new(),
    });

    let port =
        env::var("PORT").map(|p| p.parse::<u16>().expect("Failed to parse port")).unwrap_or(3000);

    info!("Server listening at localhost:{}", port);

    HttpServer::new(move || {
        let static_files = generate();

        App::new()
            .wrap(Compress::default())
            .wrap(middlewares::Auth)
            .wrap(TracingLogger::default())
            .wrap(Governor::new(&governor_config))
            .app_data(state.clone())
            .service(routes::screenshot_route)
            .service(routes::get_screenshot)
            .service(routes::schema_route)
            .service(routes::index_route)
            .service(ResourceFiles::new("/", static_files))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    client.close().await?;

    Ok(())
}
