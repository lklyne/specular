//! Which space folder a launch opens.
//!
//! The Electron app keeps the user's choice as `spacePath` in its
//! `preferences.json`. This app reads that file and never writes it, so
//! both open the same folder. A folder chosen here with File > Open space…
//! is remembered in this app's own preferences and used only while the
//! Electron app names none.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::Value;

const ELECTRON_APP_NAME: &str = "Specular";
const ELECTRON_PREFERENCES: &str = "preferences.json";
const SPACE_PATH_KEY: &str = "spacePath";
/// Where the Electron app kept the one space before the folder could be
/// chosen, under its data folder. Still its space while `spacePath` is
/// unset.
const LEGACY_SPACE: &str = "workspaces/default";

/// What to open at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpaceStart {
    /// The space folder.
    pub(crate) folder: PathBuf,
    /// The canvas file to show, by name, when the launch named one.
    /// Otherwise the space's last active canvas.
    pub(crate) file: Option<String>,
}

/// The Electron app's data folder: Application Support on macOS, else the
/// XDG config folder.
pub(crate) fn electron_user_data(
    variable: impl Fn(&str) -> Option<OsString>,
    macos: bool,
) -> Option<PathBuf> {
    let set = |name: &str| variable(name).filter(|value| !value.is_empty());
    let home = set("HOME").map(PathBuf::from);
    if macos {
        return Some(
            home?
                .join("Library/Application Support")
                .join(ELECTRON_APP_NAME),
        );
    }
    let config =
        (set("XDG_CONFIG_HOME").map(PathBuf::from)).or(home.map(|home| home.join(".config")));
    Some(config?.join(ELECTRON_APP_NAME))
}

/// The space the Electron app opens, given its data folder: the
/// `spacePath` in its preferences, else its old fixed folder if that is
/// there. `None` when it has neither.
pub(crate) fn electron_space(user_data: &Path) -> Option<PathBuf> {
    let chosen = std::fs::read_to_string(user_data.join(ELECTRON_PREFERENCES))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|preferences| Some(preferences.get(SPACE_PATH_KEY)?.as_str()?.to_owned()))
        .filter(|path| !path.is_empty());
    if let Some(chosen) = chosen {
        return Some(PathBuf::from(chosen));
    }
    let legacy = user_data.join(LEGACY_SPACE);
    legacy.is_dir().then_some(legacy)
}

/// The space a launch opens. A path on the command line wins: a folder is
/// the space, and a `.canvas` file is shown in the space its folder is.
/// With no path it is the Electron app's space, else the folder this app
/// last chose. A folder from settings that is not there is not opened:
/// an unmounted drive or a renamed folder is a question for the user, and
/// making an empty space in its place would answer it for them.
pub(crate) fn startup(
    argument: Option<&Path>,
    electron: Option<PathBuf>,
    remembered: Option<PathBuf>,
) -> Option<SpaceStart> {
    if let Some(path) = argument {
        return Some(from_argument(path));
    }
    let folder = electron.or(remembered)?;
    if !folder.is_dir() {
        tracing::warn!(
            folder = %folder.display(),
            "the space folder in settings is not there; choose one with File > Open space\u{2026}"
        );
        return None;
    }
    Some(SpaceStart { folder, file: None })
}

fn from_argument(path: &Path) -> SpaceStart {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    let is_canvas = absolute
        .extension()
        .is_some_and(|extension| extension == "canvas");
    if absolute.is_dir() || !is_canvas {
        return SpaceStart {
            folder: absolute,
            file: None,
        };
    }
    SpaceStart {
        folder: (absolute.parent()).map_or_else(|| PathBuf::from("."), Path::to_owned),
        file: (absolute.file_name()).map(|name| name.to_string_lossy().into_owned()),
    }
}
