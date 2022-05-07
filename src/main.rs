#![feature(let_chains, pattern)]

#[macro_use]
extern crate tracing;

use std::env;
use std::process::Stdio;
use std::sync::Arc;

use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::Compress;
use actix_web::{web, App, Error, HttpServer};
use fantoccini::{Client, ClientBuilder};
use portpicker::pick_unused_port;
use providers::{Provider, Storage};
use reqwest::Client as ReqwestClient;
use serde_json::Map;
use tokio::process::Command;
use tokio_process_stream::ProcessLineStream;
use tokio_stream::StreamExt;
use tracing_actix_web::TracingLogger;
use util::{initialize_tracing, load_env};

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

    let driver_port = pick_unused_port().expect("No port available");

    tokio::spawn(async move {
        let mut chromedriver = Command::new("chromedriver");
        chromedriver.stdout(Stdio::piped()).stderr(Stdio::piped());
        chromedriver.arg("--whitelisted-ips=");
        chromedriver.arg(format!("--port={driver_port}"));

        let mut stream =
            ProcessLineStream::try_from(chromedriver).expect("Failed to convert command to stream");

        while let Some(log) = stream.next().await {
            info!(target: "chromedriver", "{}", log);
        }
    });

    let debug_port = pick_unused_port().expect("No port available");
    let mut capabilities = Map::new();
    let chrome_opts = serde_json::json!({
        "args": [
            "--disable-gpu",
            "--no-sandbox",
            "--disable-dev-shm-usage",
            "--headless",
            "--verbose", 
            "--remote-debugging-address=0.0.0.0",
            format!("--remote-debugging-port={debug_port}")
        ]
    });

    capabilities.insert("goog:chromeOptions".to_owned(), chrome_opts);

    let client = ClientBuilder::rustls()
        .capabilities(capabilities)
        .connect(&format!("http://localhost:{driver_port}"))
        .await?;

    info!("Connected to chromedriver at localhost:{driver_port}");

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
