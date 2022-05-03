use std::path::Path;

use anyhow::Result;
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use regress::Regex;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::FmtSubscriber;

static URL_REGEX: OnceCell<Regex> = OnceCell::new();

fn get_url_regex() -> Result<Regex> {
    let re = URL_REGEX
        .get_or_try_init(|| {
            let re = r"(https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|www\.[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9]+\.[^\s]{2,}|www\.[a-zA-Z0-9]+\.[^\s]{2,})";

            Regex::with_flags(re, "i")
        })?
        .to_owned();

    Ok(re)
}

pub fn check_if_url(url: &str) -> Result<bool> {
    let re = get_url_regex()?;

    match re.find(url).is_some() {
        true => Ok(true),
        false => Err(anyhow::anyhow!("url not valid")),
    }
}

pub fn load_env() {
    let file_exists = Path::new(".env").exists();

    if file_exists {
        dotenv().ok();
    }
}

pub fn initialize_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_span_events(FmtSpan::CLOSE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to initialize logger");
}
