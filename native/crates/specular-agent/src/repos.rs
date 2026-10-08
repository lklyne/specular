//! Connected repos and the origins bound to them: which local folder a
//! site's source lives in, and whether a comment on that site sends itself.
//!
//! The shape is the Electron app's `repos.json` (`dev-server-manager.ts`),
//! so both apps read one file. An origin is bound to at most one repo.

use std::fmt::Write as _;

use serde_json::{Map, Value, json};
use sha2::{Digest as _, Sha256};

/// An origin bound to a repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundOrigin {
    /// `scheme://host[:port]`.
    pub origin: String,
    /// Whether a comment on this origin is sent the moment it is placed.
    pub auto_fix: bool,
}

/// A local folder connected as a repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// The first 16 hex digits of the SHA-256 of `absolute_path`.
    pub id: String,
    /// The folder.
    pub absolute_path: String,
    /// What the settings call it: the folder's name unless the file says
    /// otherwise.
    pub label: String,
    /// The origins that write to it.
    pub bound_origins: Vec<BoundOrigin>,
}

/// Where an origin writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding<'a> {
    /// The repo's folder.
    pub repo_path: &'a str,
    /// Whether comments on the origin send themselves.
    pub auto_fix: bool,
}

/// Every connected repo, in the file's order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Repos {
    items: Vec<Repo>,
}

/// The origin of `url`, as a browser's `new URL(url).origin` gives it for a
/// site: scheme, host and a port that is not the scheme's own. `None` for an
/// address with no origin to bind (`data:`, `file:`, `about:`) and for text
/// that is not a URL.
pub fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.trim().split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    let default_port = match scheme.as_str() {
        "http" | "ws" => "80",
        "https" | "wss" => "443",
        _ => return None,
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    let host = host.to_ascii_lowercase();
    let host = host
        .strip_suffix(&format!(":{default_port}"))
        .unwrap_or(&host);
    let host = host.trim_end_matches(':');
    (!host.is_empty()).then(|| format!("{scheme}://{host}"))
}

/// `normalizeOrigin`: the origin of a URL, else the text without trailing
/// slashes.
fn normalize(origin: &str) -> String {
    origin_of(origin).unwrap_or_else(|| origin.trim().trim_end_matches('/').to_owned())
}

fn id_for(absolute_path: &str) -> String {
    let digest = Sha256::digest(absolute_path.as_bytes());
    (digest.iter().take(8)).fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

fn folder_name(absolute_path: &str) -> String {
    let trimmed = absolute_path.trim_end_matches('/');
    trimmed.rsplit('/').next().unwrap_or(trimmed).to_owned()
}

impl Repos {
    /// Reads `repos.json`. Text that is not the file's shape reads as no
    /// repos; an entry with no id or no path is skipped.
    pub fn from_json(text: &str) -> Self {
        let parsed: Value = serde_json::from_str(text).unwrap_or(Value::Null);
        let list = parsed.get("repos").and_then(Value::as_array);
        let text_of = |entry: &Value, key: &str| {
            entry
                .get(key)
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(str::to_owned)
        };
        let items = list
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let (id, path) = (text_of(entry, "id")?, text_of(entry, "absolutePath")?);
                let bound = entry.get("boundOrigins").and_then(Value::as_array);
                let bound_origins = bound
                    .into_iter()
                    .flatten()
                    .filter_map(|b| {
                        Some(BoundOrigin {
                            origin: normalize(b.get("origin")?.as_str()?),
                            auto_fix: b.get("autoFix").and_then(Value::as_bool).unwrap_or(false),
                        })
                    })
                    .collect();
                Some(Repo {
                    label: text_of(entry, "label").unwrap_or_else(|| folder_name(&path)),
                    id,
                    absolute_path: path,
                    bound_origins,
                })
            })
            .collect();
        Self { items }
    }

    /// The text of `repos.json`, as the Electron app writes it: two-space
    /// indent, and no `boundOrigins` key on a repo with none.
    pub fn to_json(&self) -> String {
        let repos: Vec<Value> = (self.items.iter())
            .map(|repo| {
                let mut entry = Map::new();
                entry.insert("id".into(), json!(repo.id));
                entry.insert("absolutePath".into(), json!(repo.absolute_path));
                entry.insert("label".into(), json!(repo.label));
                if !repo.bound_origins.is_empty() {
                    let bound: Vec<Value> = (repo.bound_origins.iter())
                        .map(|b| json!({ "origin": b.origin, "autoFix": b.auto_fix }))
                        .collect();
                    entry.insert("boundOrigins".into(), Value::Array(bound));
                }
                Value::Object(entry)
            })
            .collect();
        serde_json::to_string_pretty(&json!({ "repos": repos })).unwrap_or_default()
    }

    /// Every repo.
    pub fn all(&self) -> &[Repo] {
        &self.items
    }

    /// Where `origin` writes, when it is bound.
    pub fn binding(&self, origin: &str) -> Option<Binding<'_>> {
        let origin = normalize(origin);
        self.items.iter().find_map(|repo| {
            let bound = repo.bound_origins.iter().find(|b| b.origin == origin)?;
            Some(Binding {
                repo_path: &repo.absolute_path,
                auto_fix: bound.auto_fix,
            })
        })
    }

    /// Connects the folder at `absolute_path`, or finds it already
    /// connected. Returns the repo's id.
    pub fn connect(&mut self, absolute_path: &str) -> String {
        let id = id_for(absolute_path);
        if !self.items.iter().any(|repo| repo.id == id) {
            self.items.push(Repo {
                id: id.clone(),
                absolute_path: absolute_path.to_owned(),
                label: folder_name(absolute_path),
                bound_origins: Vec::new(),
            });
        }
        id
    }

    /// Binds `origin` to the repo at `absolute_path`, connecting it first
    /// when needed. Whatever the origin was bound to before is dropped, and
    /// auto-fix starts off.
    pub fn bind(&mut self, origin: &str, absolute_path: &str) {
        let id = self.connect(absolute_path);
        self.bind_to(&id, origin);
    }

    /// Binds `origin` to a connected repo. Returns whether the repo exists.
    pub fn bind_to(&mut self, repo_id: &str, origin: &str) -> bool {
        let origin = normalize(origin);
        if origin.is_empty() || !self.items.iter().any(|repo| repo.id == repo_id) {
            return false;
        }
        self.unbind(&origin);
        if let Some(repo) = self.items.iter_mut().find(|repo| repo.id == repo_id) {
            repo.bound_origins.push(BoundOrigin {
                origin,
                auto_fix: false,
            });
        }
        true
    }

    /// Drops the binding of `origin`. Returns whether there was one.
    pub fn unbind(&mut self, origin: &str) -> bool {
        let origin = normalize(origin);
        let mut removed = false;
        for repo in &mut self.items {
            let before = repo.bound_origins.len();
            repo.bound_origins.retain(|b| b.origin != origin);
            removed |= repo.bound_origins.len() != before;
        }
        removed
    }

    /// Turns auto-fix on or off for a bound origin. Returns whether anything
    /// changed.
    pub fn set_auto_fix(&mut self, origin: &str, on: bool) -> bool {
        let origin = normalize(origin);
        let mut changed = false;
        for bound in (self.items.iter_mut()).flat_map(|repo| repo.bound_origins.iter_mut()) {
            if bound.origin == origin && bound.auto_fix != on {
                bound.auto_fix = on;
                changed = true;
            }
        }
        changed
    }

    /// Disconnects a repo and drops its bindings. Returns whether it was
    /// connected.
    pub fn disconnect(&mut self, repo_id: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|repo| repo.id != repo_id);
        self.items.len() != before
    }
}
