//! What the owner's machine keeps about its shares, in
//! `~/.specular/share.json`: the owner's session on each server and which
//! doc each canvas file was published as. It is never kept next to the
//! canvas, so a shared folder never carries the credential. The format is
//! the one `server/scripts/share.ts` writes, so the two share it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The contents of `share.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct Account {
    /// Server URL to the owner's session cookie there.
    #[serde(default)]
    pub(super) sessions: BTreeMap<String, String>,
    /// Absolute canvas path to where it was published.
    #[serde(default)]
    pub(super) canvases: BTreeMap<String, Published>,
}

/// Where one canvas was published.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Published {
    pub(super) server: String,
    #[serde(rename = "docId")]
    pub(super) doc_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) title: Option<String>,
}

impl Account {
    fn file() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| Path::new(&home).join(".specular/share.json"))
    }

    /// The kept account, or an empty one when there is none yet.
    pub(super) fn load() -> Self {
        let text = Self::file().and_then(|file| std::fs::read_to_string(file).ok());
        text.and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Keeps the account, readable by the owner alone.
    pub(super) fn save(&self) -> anyhow::Result<()> {
        let file = Self::file().ok_or_else(|| anyhow::anyhow!("no home folder"))?;
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(&file, serde_json::to_string_pretty(self)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}
