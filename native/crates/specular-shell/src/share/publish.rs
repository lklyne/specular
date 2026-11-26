//! Publishing a canvas to the share server: sign in once, make the canvas a
//! doc, upload its file and the files it names, record a version, and make
//! the three links people open it with. It runs on its own thread; the
//! server's HTTP surface is the one `server/scripts/share.ts` uses.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use anyhow::{Context as _, anyhow};
use sha2::{Digest as _, Sha256};
use specular_interact::ShareLinks;

use super::account::{Account, Published};

/// The server a share goes to when `SPECULAR_SHARE_SERVER` names none.
const DEFAULT_SERVER: &str = "https://share.specular.sh";

/// The server shares go to now.
pub(crate) fn server() -> String {
    std::env::var("SPECULAR_SHARE_SERVER")
        .ok()
        .filter(|server| !server.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SERVER.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

/// One canvas to publish.
#[derive(Debug, Clone)]
pub(crate) struct Publish {
    /// The canvas file, just written.
    pub(crate) file: PathBuf,
    /// What the shared canvas is called.
    pub(crate) title: String,
    /// The server, `https://host` with no trailing slash.
    pub(crate) server: String,
}

/// Publishes on a thread of its own; the answer comes on the receiver.
pub(crate) fn start(publish: Publish) -> Receiver<Result<ShareLinks, String>> {
    let (answer, answered) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("share-publish".to_owned())
        .spawn({
            let answer = answer.clone();
            move || {
                let result = run(&publish).map_err(|error| format!("{error:#}"));
                let _ = answer.send(result);
            }
        });
    if let Err(error) = spawned {
        let _ = answer.send(Err(format!("cannot start publishing: {error}")));
    }
    answered
}

fn run(publish: &Publish) -> anyhow::Result<ShareLinks> {
    let Publish {
        file,
        title,
        server,
    } = publish;
    let key = file.to_string_lossy().into_owned();
    let mut account = Account::load();
    let cookie = session(&mut account, server)?;
    let known = (account.canvases.get(&key)).filter(|known| known.server == *server);
    let doc = if let Some(known) = known {
        known.doc_id.clone()
    } else {
        let created = json(post(&format!("{server}/docs"), &cookie, server).send_empty())?;
        let doc = text_field(&created, "docId")?;
        account.canvases.insert(
            key.clone(),
            Published {
                server: server.clone(),
                doc_id: doc.clone(),
                title: Some(title.clone()),
            },
        );
        account.save().context("keep the share")?;
        doc
    };
    let connect = post(&format!("{server}/docs/{doc}/connect"), &cookie, server).send_empty();
    let token = text_field(&json(connect)?, "token")?;

    let text = std::fs::read(file).with_context(|| format!("read {}", file.display()))?;
    let canvas: serde_json::Value = serde_json::from_slice(&text).context("read the canvas")?;
    let folder = file.parent().unwrap_or(Path::new("."));
    let mut files = serde_json::Map::new();
    for name in files_of(&canvas) {
        let path = folder.join(&name);
        match std::fs::read(&path) {
            Ok(bytes) => {
                let asset = upload(server, &doc, &token, &bytes, content_type(&name))?;
                files.insert(name, asset.into());
            }
            Err(error) => tracing::warn!("{name} is not shared: {error}"),
        }
    }
    let manifest = serde_json::json!({
        "format": 1,
        "title": title,
        "publishedAt": now_iso(),
        "canvas": upload(server, &doc, &token, &text, "application/json")?,
        // Page snapshots come from the page hosts, which the publish thread
        // cannot reach; the web viewer shows a page with none as its card.
        "pages": {},
        "files": files,
    });
    let versions = ureq::post(format!("{server}/docs/{doc}/versions"))
        .header("authorization", format!("Bearer {token}"))
        .content_type("application/json")
        .send(manifest.to_string().as_str());
    answered(versions)?;

    let link = |scope: &str| -> anyhow::Result<String> {
        let body = serde_json::json!({ "scope": scope }).to_string();
        let made = post(&format!("{server}/docs/{doc}/links"), &cookie, server)
            .content_type("application/json")
            .send(body.as_str());
        Ok(format!("{server}{}", text_field(&json(made)?, "url")?))
    };
    Ok(ShareLinks {
        edit: link("edit")?,
        comment: link("comment")?,
        view: link("view")?,
    })
}

type Request = ureq::RequestBuilder<ureq::typestate::WithBody>;
type Answer = Result<ureq::http::Response<ureq::Body>, ureq::Error>;

/// A request the owner's session makes, from the server's own origin as
/// the server's auth asks.
fn post(url: &str, cookie: &str, server: &str) -> Request {
    ureq::post(url)
        .header("cookie", cookie)
        .header("origin", server)
}

/// The owner's session cookie on `server`, signing in anonymously once.
fn session(account: &mut Account, server: &str) -> anyhow::Result<String> {
    if let Some(cookie) = account.sessions.get(server) {
        return Ok(cookie.clone());
    }
    let signed = ureq::post(format!("{server}/api/auth/sign-in/anonymous"))
        .header("origin", server)
        .content_type("application/json")
        .send("{}");
    let signed = answered(signed)?;
    let cookie = (signed.headers().get_all("set-cookie").iter())
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    if cookie.is_empty() {
        return Err(anyhow!("the server gave no session"));
    }
    account.sessions.insert(server.to_owned(), cookie.clone());
    account.save().context("keep the session")?;
    Ok(cookie)
}

/// Uploads `bytes` unless the server has them already; their asset id.
fn upload(
    server: &str,
    doc: &str,
    token: &str,
    bytes: &[u8],
    content_type: &str,
) -> anyhow::Result<String> {
    let mut hash = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(hash, "{byte:02x}");
    }
    if ureq::head(format!("{server}/assets/{hash}")).call().is_ok() {
        return Ok(hash);
    }
    let sent = ureq::post(format!("{server}/docs/{doc}/assets"))
        .header("authorization", format!("Bearer {token}"))
        .content_type(content_type)
        .send(bytes);
    text_field(&json(sent)?, "assetId")
}

fn answered(answer: Answer) -> anyhow::Result<ureq::http::Response<ureq::Body>> {
    answer.map_err(|error| match error {
        ureq::Error::StatusCode(code) => anyhow!("the share server answered {code}"),
        error => anyhow!("cannot reach the share server: {error}"),
    })
}

fn json(answer: Answer) -> anyhow::Result<serde_json::Value> {
    let mut answer = answered(answer)?;
    let text = answer.body_mut().read_to_string()?;
    Ok(serde_json::from_str(&text)?)
}

fn text_field(value: &serde_json::Value, field: &str) -> anyhow::Result<String> {
    (value.get(field).and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("the share server gave no {field}"))
}

/// The files a canvas names that live on disk: images and Documents.
fn files_of(canvas: &serde_json::Value) -> Vec<String> {
    let nodes = (canvas.get("nodes").and_then(serde_json::Value::as_array)).into_iter();
    let mut files: Vec<String> = (nodes.flatten())
        .filter_map(|node| node.get("file")?.as_str())
        .filter(|file| !file.contains("://"))
        .map(str::to_owned)
        .collect();
    files.sort();
    files.dedup();
    files
}

fn content_type(file: &str) -> &'static str {
    let extension = file.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "md" | "markdown" => "text/markdown; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Now as an ISO 8601 UTC time.
fn now_iso() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        });
    specular_interact::iso8601(ms)
}

#[cfg(test)]
mod tests {
    use super::files_of;

    #[test]
    fn a_canvas_names_its_local_files_once() {
        let canvas = serde_json::json!({ "nodes": [
            { "type": "file", "file": "b.png" },
            { "type": "file", "file": "a.md" },
            { "type": "file", "file": "b.png" },
            { "type": "file", "file": "https://x/c.png" },
            { "type": "link", "url": "https://x" },
        ]});
        assert_eq!(files_of(&canvas), ["a.md", "b.png"]);
    }
}
