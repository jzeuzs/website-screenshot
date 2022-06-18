use std::ops::Not;
use std::path::Path;
use std::str::pattern::{Pattern, SearchStep, Searcher};

use anyhow::Result;
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use regress::{Matches, Regex};
use tokio::sync;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::FmtSubscriber;

static URL_REGEX: OnceCell<Regex> = OnceCell::new();
static NSFW_SITE_LIST: sync::OnceCell<Vec<String>> = sync::OnceCell::const_new();

async fn get_nsfw_list<'a>() -> Result<&'a Vec<String>> {
    let list = NSFW_SITE_LIST
        .get_or_init(|| async {
            let text = reqwest::get("https://blocklistproject.github.io/Lists/porn.txt")
                .await
                .expect("Failed fetching nsfw list")
                .text()
                .await
                .expect("Failed converting into text");

            text.split('\n')
                .par_bridge()
                .filter(|s| s.is_empty().not() && s.starts_with('#').not())
                .map(|s| {
                    s.replace(
                        RegexPattern(&Regex::new("^(0.0.0.0 )").expect("Failed compiling regex")),
                        "",
                    )
                })
                .collect()
        })
        .await;

    Ok(list)
}

pub async fn check_if_nsfw(host: &str) -> Result<bool> {
    let list = get_nsfw_list().await?;

    Ok(list.par_iter().any(|s| s == host))
}

fn get_url_regex<'a>() -> Result<&'a Regex> {
    let re = URL_REGEX
        .get_or_try_init(|| {
            let re = r"(https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|www\.[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9]+\.[^\s]{2,}|www\.[a-zA-Z0-9]+\.[^\s]{2,})";

            Regex::with_flags(re, "i")
        })?;

    Ok(re)
}

pub fn check_if_url(url: &str) -> Result<bool> {
    let re = get_url_regex()?;

    if re.find(url).is_some() {
        Ok(true)
    } else {
        Err(anyhow::anyhow!("url not valid"))
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

/// Test utilities
pub mod test {
    use std::sync::Arc;

    use actix_web::web::Data;
    use fantoccini::{Client, ClientBuilder};
    use once_cell::sync::Lazy;
    use serde_json::{json, Map, Value};

    use super::Result;
    use crate::providers::Provider;
    use crate::{State, Storage};

    #[derive(Debug)]
    pub struct ResponseError<'a>(pub &'a dyn actix_web::error::ResponseError);

    impl PartialEq for ResponseError<'_> {
        fn eq(&self, rhs: &Self) -> bool {
            self.0.status_code() == rhs.0.status_code()
        }
    }

    static CAPABILITIES: Lazy<Map<String, Value>> = Lazy::new(|| {
        let mut capabilities = Map::new();
        let chrome_opts = json!({
            "args": [
                "--disable-gpu",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--headless",
                "--whitelisted-ips="
            ]
        });

        capabilities.insert("goog:chromeOptions".to_owned(), chrome_opts);
        capabilities
    });

    pub async fn setup_with_state() -> Result<(Client, Data<State>)> {
        let client = ClientBuilder::rustls()
            .capabilities(Lazy::force(&CAPABILITIES).clone())
            .connect("http://localhost:9515")
            .await?;

        let state = Data::new(State {
            browser: Arc::new(client.clone()),
            storage: Arc::new(Storage::new()),
            reqwest: reqwest::Client::new(),
        });

        Ok((client, state))
    }
}

pub struct RegexSearcher<'r, 't> {
    haystack: &'t str,
    it: Matches<'r, 't>,
    last_step_end: usize,
    next_match: Option<(usize, usize)>,
}
pub struct RegexPattern<'r>(&'r Regex);

impl<'r, 't> Pattern<'t> for RegexPattern<'r> {
    type Searcher = RegexSearcher<'r, 't>;

    fn into_searcher(self, haystack: &'t str) -> RegexSearcher<'r, 't> {
        RegexSearcher {
            haystack,
            it: self.0.find_iter(haystack),
            last_step_end: 0,
            next_match: None,
        }
    }
}

unsafe impl<'r, 't> Searcher<'t> for RegexSearcher<'r, 't> {
    #[inline]
    fn haystack(&self) -> &'t str {
        self.haystack
    }

    #[inline]
    fn next(&mut self) -> SearchStep {
        if let Some((s, e)) = self.next_match {
            self.next_match = None;
            self.last_step_end = e;

            return SearchStep::Match(s, e);
        }
        match self.it.next() {
            None => {
                if self.last_step_end < self.haystack().len() {
                    let last = self.last_step_end;
                    self.last_step_end = self.haystack().len();

                    SearchStep::Reject(last, self.haystack().len())
                } else {
                    SearchStep::Done
                }
            },
            Some(m) => {
                let (s, e) = (m.start(), m.end());
                if s == self.last_step_end {
                    self.last_step_end = e;

                    SearchStep::Match(s, e)
                } else {
                    self.next_match = Some((s, e));
                    let last = self.last_step_end;

                    self.last_step_end = s;

                    SearchStep::Reject(last, s)
                }
            },
        }
    }
}
