#![feature(let_chains, pattern)]

#[macro_use]
extern crate tracing;

use std::env;
use std::process::Stdio;
use std::sync::Arc;

use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::Compress;
use actix_web::{web, App, Error, HttpServer};
use actix_web_static_files::ResourceFiles;
use cdp::ChromeCommand;
use evasions::*;
use fantoccini::{Client, ClientBuilder};
use providers::{Provider, Storage};
use reqwest::Client as ReqwestClient;
use serde_json::{Map, Value};
use tokio::process::Command;
use tokio_process_stream::ProcessLineStream;
use tokio_stream::StreamExt;
use tracing_actix_web::TracingLogger;
use util::{initialize_tracing, load_env};

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

    tokio::spawn(async move {
        let path = match env::var("CHROMEDRIVER_PATH") {
            Ok(path) => path,
            Err(_) => "chromedriver".to_owned(),
        };

        let mut chromedriver = Command::new(path);
        chromedriver.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut stream =
            ProcessLineStream::try_from(chromedriver).expect("Failed to convert command to stream");

        while let Some(log) = stream.next().await {
            info!(target: "chromedriver", "{}", log);
        }
    });

    let mut capabilities = Map::new();
    let chrome_opts = match env::var("GOOGLE_CHROME_PATH") {
        Ok(path) => serde_json::json!({
            "binary": path,
            "args": [
                "--disable-gpu",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--headless",
                "--whitelisted-ips="
            ]
        }),
        Err(_) => serde_json::json!({
            "args": [
                "--disable-gpu",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--headless",
                "--whitelisted-ips="
            ]
        }),
    };

    capabilities.insert("goog:chromeOptions".to_owned(), chrome_opts);

    let client =
        ClientBuilder::rustls().capabilities(capabilities).connect("http://localhost:9515").await?;

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
