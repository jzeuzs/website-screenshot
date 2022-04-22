use std::path::Path;

use dotenv::dotenv;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::FmtSubscriber;

pub fn load_env() {
    let file_exists = Path::new(".env").exists();

    if file_exists {
        dotenv().ok();
    }
}

pub fn initialize_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to initialize logger");
}
