/// MIT License
///
/// Copyright (c) 2019-2021 Stephen Pryde and the thirtyfour contributors
///
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
///
/// The above copyright notice and this permission notice shall be included in all
/// copies or substantial portions of the Software.
///
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
/// SOFTWARE.
use fantoccini::wd::WebDriverCompatibleCommand;
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::{ParseError, Url};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename = "lowercase")]
pub enum ConnectionType {
    None,
    Cellular2G,
    Cellular3G,
    Cellular4G,
    Bluetooth,
    Ethernet,
    Wifi,
    Wimax,
    Other,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct NetworkConditions {
    pub offline: bool,
    pub latency: u32,
    pub download_throughput: i32,
    pub upload_throughput: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<ConnectionType>,
}

#[derive(Debug)]
pub enum ChromeCommand {
    LaunchApp(String),
    GetNetworkConditions,
    SetNetworkConditions(NetworkConditions),
    ExecuteCdpCommand(String, Value),
    GetSinks,
    GetIssueMessage,
    SetSinkToUse(String),
    StartTabMirroring(String),
    StopCasting(String),
}

impl WebDriverCompatibleCommand for ChromeCommand {
    fn endpoint(&self, base_url: &Url, session_id: Option<&str>) -> Result<Url, ParseError> {
        let base = { base_url.join(&format!("session/{}/", session_id.as_ref().unwrap()))? };
        match &self {
            ChromeCommand::LaunchApp(_) => base.join("chromium/launch_app"),
            ChromeCommand::GetNetworkConditions | ChromeCommand::SetNetworkConditions(_) => {
                base.join("chromium/network_conditions")
            },
            ChromeCommand::ExecuteCdpCommand(..) => base.join("goog/cdp/execute"),
            ChromeCommand::GetSinks => base.join("goog/cast/get_sinks"),
            ChromeCommand::GetIssueMessage => base.join("goog/cast/get_issue_message"),
            ChromeCommand::SetSinkToUse(_) => base.join("goog/cast/set_sink_to_use"),
            ChromeCommand::StartTabMirroring(_) => base.join("goog/cast/start_tab_mirroring"),
            ChromeCommand::StopCasting(_) => base.join("goog/cast/stop_casting"),
        }
    }

    fn method_and_body(&self, _request_url: &Url) -> (Method, Option<String>) {
        let mut method = Method::GET;
        let mut body = None;

        match &self {
            ChromeCommand::LaunchApp(app_id) => {
                method = Method::POST;
                body = Some(json!({ "id": app_id }).to_string());
            },
            ChromeCommand::SetNetworkConditions(conditions) => {
                method = Method::POST;
                body = Some(json!({ "network_conditions": conditions }).to_string());
            },
            ChromeCommand::ExecuteCdpCommand(command, params) => {
                method = Method::POST;
                body = Some(json!({"cmd": command, "params": params }).to_string());
            },
            ChromeCommand::SetSinkToUse(sink_name)
            | ChromeCommand::StartTabMirroring(sink_name)
            | ChromeCommand::StopCasting(sink_name) => {
                method = Method::POST;
                body = Some(json!({ "sinkName": sink_name }).to_string());
            },
            _ => {},
        }

        (method, body)
    }
}
