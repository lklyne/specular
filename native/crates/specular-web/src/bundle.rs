//! [`Manifest`]: what a published canvas is on the server. The desktop
//! writes it when it publishes and the web page reads it before it builds a
//! [`Viewer`](crate::Viewer). Every file it names is a content-addressed
//! asset, `GET /assets/<hash>`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The manifest format this build writes and reads.
pub const FORMAT: u32 = 1;

/// One published version of a canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// [`FORMAT`] when it was written.
    pub format: u32,
    /// The canvas's name, for the page title.
    pub title: String,
    /// When it was published, ISO 8601.
    pub published_at: String,
    /// The `.canvas` text, as an asset hash.
    pub canvas: String,
    /// Each page's last capture: the page entity's id to the PNG.
    #[serde(default)]
    pub pages: BTreeMap<String, Snapshot>,
    /// The images and markdown files the canvas names, by the path it
    /// writes them with, to their asset hashes.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

/// One page as it looked when the canvas was published.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// The PNG, as an asset hash.
    pub asset: String,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_reads_what_the_publisher_writes() {
        let text = r#"{
            "format": 1, "title": "Checkout", "publishedAt": "2026-10-27T00:00:00.000Z",
            "canvas": "c0ffee",
            "pages": {"p1": {"asset": "beef", "width": 2560, "height": 1600}},
            "files": {"assets/shot.png": "f00d"}
        }"#;
        let manifest: Manifest = serde_json::from_str(text).unwrap();
        assert_eq!(manifest.pages["p1"].width, 2560);
        assert_eq!(manifest.files["assets/shot.png"], "f00d");
        let again: Manifest =
            serde_json::from_str(&serde_json::to_string(&manifest).unwrap()).unwrap();
        assert_eq!(again, manifest);
    }
}
