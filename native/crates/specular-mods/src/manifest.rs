//! `mod.json`: our own manifest, beside `main.js`.
//!
//! `gpui-shell` has a manifest of its own (`gpui-shell.json`) that refuses
//! keys it does not know, so it cannot carry ours. A mod has no
//! `gpui-shell.json` and is loaded from `main.js`.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The mod API generation this build of the app speaks. Within a generation
/// the surface (command names and their arguments, state keys, the
/// `specular` module's functions) only grows. A break is a new generation.
pub const API: u32 = 1;

/// The file a mod's manifest is read from.
pub const FILE: &str = "mod.json";

/// A palette entry a mod adds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaletteEntry {
    /// What the palette shows.
    pub title: String,
    /// The command it runs.
    pub run: String,
    /// The command's arguments.
    #[serde(default)]
    pub args: Value,
}

/// A mod's `mod.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModManifest {
    /// The API generation the mod was written for.
    pub api: u32,
    /// What the mod is called where a person sees it.
    pub title: String,
    /// The region it goes in when no layout names it.
    #[serde(default)]
    pub place: Option<String>,
    /// Entries it adds to the palette.
    #[serde(default)]
    pub palette: Vec<PaletteEntry>,
    /// The shipped mod this one was cloned from, written by `specular mod
    /// clone`. A clone is never replaced on start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloned_from: Option<String>,
}

/// Why a mod's manifest was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    /// The file.
    pub path: PathBuf,
    /// What is wrong with it.
    pub problem: String,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.problem)
    }
}

impl std::error::Error for ManifestError {}

impl ModManifest {
    /// Parses `source`, refusing a generation newer than [`API`].
    pub fn parse(source: &str, path: &Path) -> Result<Self, ManifestError> {
        let refuse = |problem: String| ManifestError {
            path: path.to_path_buf(),
            problem,
        };
        let manifest: Self = serde_json::from_str(source).map_err(|e| refuse(e.to_string()))?;
        if manifest.api > API {
            return Err(refuse(format!(
                "written for mod API {}, and this Specular speaks {API}: update Specular",
                manifest.api
            )));
        }
        if manifest.api == 0 {
            return Err(refuse("`api` starts at 1".to_owned()));
        }
        Ok(manifest)
    }

    /// Reads `dir/mod.json`.
    pub fn read(dir: &Path) -> Result<Self, ManifestError> {
        let path = dir.join(FILE);
        let source = std::fs::read_to_string(&path).map_err(|e| ManifestError {
            path: path.clone(),
            problem: e.to_string(),
        })?;
        Self::parse(&source, &path)
    }
}
