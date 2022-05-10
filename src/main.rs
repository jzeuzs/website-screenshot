#![feature(let_chains, pattern)]

#[macro_use]
extern crate tracing;

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs, thread};

use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::Compress;
use actix_web::{web, App, Error, HttpServer};
use cdp::ChromeCommand;
use fantoccini::{Client, ClientBuilder};
use providers::{Provider, Storage};
use reqwest::Client as ReqwestClient;
use serde_json::{Map, Value};
use tokio::process::Command;
use tokio_process_stream::ProcessLineStream;
use tokio_stream::StreamExt;
use tracing_actix_web::TracingLogger;
use util::{evaluate_on_new_document, initialize_tracing, load_env};

pub mod cdp;
pub mod error;
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

    // Chromedriver may take a while to start
    thread::sleep(Duration::from_secs(3));

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
        evaluate_on_new_document(&client, fs::read_to_string("evasions/utils.js")?, vec![]),
        evaluate_on_new_document(&client, fs::read_to_string("evasions/chrome.app.js")?, vec![]),
        evaluate_on_new_document(&client, fs::read_to_string("evasions/chrome.csi.js")?, vec![]),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/chrome.loadTimes.js")?,
            vec![]
        ),
        evaluate_on_new_document(&client, fs::read_to_string("evasions/chrome.runtime.js")?, vec![
            Value::Bool(false)
        ]),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/iframe.contentWindow.js")?,
            vec![]
        ),
        evaluate_on_new_document(&client, fs::read_to_string("evasions/media.codecs.js")?, vec![]),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/navigator.hardwareConcurrency.js")?,
            vec![Value::Number(4.into())]
        ),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/navigator.languages.js")?,
            vec![Value::Array(vec!["en-US".into(), "en".into()])]
        ),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/navigator.permissions.js")?,
            vec![]
        ),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/navigator.plugins.js")?,
            vec![]
        ),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/navigator.vendor.js")?,
            vec![Value::String("Google Inc.".to_owned())]
        ),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/navigator.webdriver.js")?,
            vec![]
        ),
        evaluate_on_new_document(&client, fs::read_to_string("evasions/webgl.vendor.js")?, vec![
            Value::String("Intel Inc.".to_owned()),
            Value::String("Intel Iris OpenGL Engine".to_owned())
        ]),
        evaluate_on_new_document(
            &client,
            fs::read_to_string("evasions/window.outerdimensions.js")?,
            vec![]
        ),
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
        .per_second(2)
        .burst_size(5)
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
        App::new()
            .wrap(Compress::default())
            .wrap(middlewares::Auth)
            .wrap(TracingLogger::default())
            .wrap(Governor::new(&governor_config))
            .app_data(state.clone())
            .service(routes::screenshot_route)
            .service(routes::get_screenshot)
            .service(routes::index_route)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    client.close().await?;

    Ok(())
}
