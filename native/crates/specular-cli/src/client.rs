//! Talking to the running app: the discovery file that says where it is,
//! the headers every request carries, and the answer's error shape.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, anyhow, bail};
use serde_json::{Value, json};
use specular_api::{DISCOVERY_FILE, Discovery, SECRET_HEADER, SESSION_HEADER, TAB_HEADER, VERSION};

use crate::session;

/// The header that names the program calling, so the app can tell two
/// agents' cursors apart.
const CLIENT_HEADER: &str = "x-specular-client-name";
/// Set to give this agent's cursor a name of its own.
const CLIENT_ENV: &str = "SPECULAR_CLIENT_NAME";
/// Long enough for a page to load behind a write; the app's server gives
/// up sooner.
const TIMEOUT: Duration = Duration::from_secs(120);
/// For bookkeeping that must never hold a verb up.
const QUICK: Duration = Duration::from_secs(2);

/// A connection to the app, set up once per command.
pub(crate) struct Client {
    agent: ureq::Agent,
    quick: ureq::Agent,
    session: String,
    name: String,
    /// The `--tab` ref, sent with every request.
    tab: Option<String>,
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(timeout))
        .build()
        .into()
}

/// `SPECULAR_DISCOVERY_FILE`, under the temp folder when relative, else
/// the shared file in `~/.specular`. The home folder rather than the temp
/// one, so a sandbox that moves `TMPDIR` still finds the app.
fn discovery_path() -> Option<PathBuf> {
    if let Some(file) = std::env::var_os("SPECULAR_DISCOVERY_FILE").filter(|f| !f.is_empty()) {
        let file = PathBuf::from(file);
        return Some(if file.is_absolute() {
            file
        } else {
            std::env::temp_dir().join(file)
        });
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".specular").join(DISCOVERY_FILE))
}

fn load_discovery() -> anyhow::Result<Discovery> {
    let read = || -> anyhow::Result<Discovery> {
        let path = discovery_path().context("no home folder to find the app from")?;
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("no discovery file at {}", path.display()))?;
        let discovery: Discovery = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a discovery file", path.display()))?;
        if discovery.version != VERSION {
            bail!(
                "App control API version mismatch. Expected {VERSION}, got {}.",
                discovery.version
            );
        }
        Ok(discovery)
    };
    read()
        .map_err(|error| anyhow!("Specular app is not available. Launch the app first. {error:#}"))
}

/// `text` as JavaScript's `encodeURIComponent` writes it: a header is
/// Latin-1 and a tab's name is free text.
fn encode_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

impl Client {
    /// A client for the app, sending `tab` as the target canvas.
    pub(crate) fn new(tab: Option<&str>) -> Self {
        Self {
            agent: agent(TIMEOUT),
            quick: agent(QUICK),
            session: session::id(),
            name: std::env::var(CLIENT_ENV).unwrap_or_else(|_| "specular-cli".to_owned()),
            tab: tab
                .map(str::trim)
                .filter(|tab| !tab.is_empty())
                .map(str::to_owned),
        }
    }

    /// This conversation's session id.
    pub(crate) fn session(&self) -> &str {
        &self.session
    }

    /// The name the app shows for this caller.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// `GET path`.
    pub(crate) fn get(&self, path: &str) -> anyhow::Result<Value> {
        self.call(&self.agent, "GET", path, None)
    }

    /// `POST path` with `body`.
    pub(crate) fn post(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        self.call(&self.agent, "POST", path, Some(body))
    }

    /// `POST path` with `body`, for bookkeeping: quick, and a failure is
    /// ignored.
    pub(crate) fn notify(&self, path: &str, body: &Value) {
        let _ = self.call(&self.quick, "POST", path, Some(body));
    }

    /// Opens this conversation's session. It is never closed: the app
    /// times it out, so a cursor stays up across a chain of commands.
    pub(crate) fn open_session(&self) {
        let body = json!({ "sessionId": self.session, "clientName": self.name });
        self.notify("/mcp/session/open", &body);
    }

    fn call(
        &self,
        agent: &ureq::Agent,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> anyhow::Result<Value> {
        let discovery = load_discovery()?;
        let url = format!("http://127.0.0.1:{}{path}", discovery.port);
        let mut request = ureq::http::Request::builder()
            .method(method)
            .uri(&url)
            .header("content-type", "application/json")
            .header(SECRET_HEADER, &discovery.secret)
            .header(SESSION_HEADER, &self.session)
            .header(CLIENT_HEADER, &self.name);
        if let Some(tab) = &self.tab {
            request = request.header(TAB_HEADER, encode_component(tab));
        }
        let text = body.map_or_else(String::new, Value::to_string);
        let request = request.body(text)?;
        let mut response = agent.run(request).map_err(|error| {
            anyhow!(
                "could not reach the app on port {}: {error}",
                discovery.port
            )
        })?;
        let status = response.status().as_u16();
        let payload: Option<Value> = response
            .body_mut()
            .read_to_string()
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        if (200..300).contains(&status) {
            return payload.context("the app's answer was not JSON");
        }
        if status == 401 {
            bail!(
                "Specular rejected the request (stale credentials). The discovery file points \
                 at a server with a different secret — quit any extra Specular instances and \
                 relaunch the app, then retry."
            );
        }
        let message = (payload.as_ref())
            .and_then(|payload| payload["error"].as_str())
            .map_or_else(|| format!("Request failed with {status}"), str::to_owned);
        bail!(message)
    }
}
