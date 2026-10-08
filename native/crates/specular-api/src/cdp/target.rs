//! The one target a [`PageProxy`](super::PageProxy) shows its clients.

use serde_json::{Value, json};

/// What a client is told when it asks to make or close a target.
pub(super) const LIFECYCLE: &str = "Specular owns page lifecycle: add a page with `specular add page <url>` and remove \
     one with `specular delete <id>`";

/// A page as a CDP client is told of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The target id: the page entity's id, which outlives a reload.
    pub id: String,
    /// The address the page shows.
    pub url: String,
    /// The page's title.
    pub title: String,
}

impl Target {
    /// The protocol's `TargetInfo` for the page.
    pub(super) fn info(&self, attached: bool) -> Value {
        json!({
            "targetId": self.id,
            "type": "page",
            "title": self.title,
            "url": self.url,
            "attached": attached,
            "canAccessOpener": false,
            "browserContextId": "specular",
        })
    }
}

/// The answer to `Browser.getVersion`. A page's own devtools agent has no
/// `Browser` domain, so the proxy answers for it.
pub(super) fn browser_version() -> Value {
    json!({
        "protocolVersion": "1.3",
        "product": "Specular/native",
        "revision": "",
        "userAgent": "",
        "jsVersion": "",
    })
}
