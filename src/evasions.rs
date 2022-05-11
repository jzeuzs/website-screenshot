use fantoccini::Client;
use serde_json::{json, Value};

use crate::cdp::ChromeCommand;

pub const CHROME_APP: &str = include_str!("../evasions/chrome.app.js");
pub const CHROME_CSI: &str = include_str!("../evasions/chrome.csi.js");
pub const CHROME_LOADTIMES: &str = include_str!("../evasions/chrome.loadTimes.js");
pub const CHROME_RUNTIME: &str = include_str!("../evasions/chrome.runtime.js");
pub const IFRAME_CONTENTWINDOW: &str = include_str!("../evasions/iframe.contentWindow.js");
pub const MEDIA_CODECS: &str = include_str!("../evasions/media.codecs.js");
pub const NAVIGATOR_HARDWARECONCURRENCY: &str =
    include_str!("../evasions/navigator.hardwareConcurrency.js");
pub const NAVIGATOR_LANGUAGES: &str = include_str!("../evasions/navigator.languages.js");
pub const NAVIGATOR_PERMISSIONS: &str = include_str!("../evasions/navigator.permissions.js");
pub const NAVIGATOR_PLUGINS: &str = include_str!("../evasions/navigator.plugins.js");
pub const NAVIGATOR_VENDOR: &str = include_str!("../evasions/navigator.vendor.js");
pub const NAVIGATOR_WEBDRIVER: &str = include_str!("../evasions/navigator.webdriver.js");
pub const UTILS: &str = include_str!("../evasions/utils.js");
pub const WEBGL_VENDOR: &str = include_str!("../evasions/webgl.vendor.js");
pub const WINDOW_OUTERDIMENSIONS: &str = include_str!("../evasions/window.outerdimensions.js");

pub async fn evaluate_on_new_document<'a>(client: &Client, js: &'a str, args: Vec<Value>) {
    let expr = format!(
        "({js})({})",
        args.into_iter().map(|arg| arg.to_string()).collect::<Vec<_>>().join(", ")
    );

    client
        .issue_cmd(ChromeCommand::ExecuteCdpCommand(
            "Page.addScriptToEvaluateOnNewDocument".to_owned(),
            json!({ "source": expr }),
        ))
        .await
        .expect("Failed issuing cmd");
}
