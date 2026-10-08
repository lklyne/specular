//! Which space folder a launch opens.
//!
//! The app opens the folder the user chose in it, which it remembers in
//! its own preferences. With none chosen, or with the chosen one gone, it
//! opens nothing and asks (ADR 0033): it never picks a folder, and never
//! makes an empty space where one went missing. The winit shell has no view
//! to ask in, so a launch of it that names no space opens the scratch
//! space: a copy of the starter space in this app's own data folder.
//!
//! The Electron app keeps its user's choice as `spacePath` in its
//! `preferences.json`. This app reads that file and never writes it. Its
//! folder is offered on a first run and opened by `--space user`, and is
//! otherwise left alone.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::Value;
use specular_interact::SpaceAsk;

const ELECTRON_APP_NAME: &str = "Specular";
const ELECTRON_PREFERENCES: &str = "preferences.json";
const SPACE_PATH_KEY: &str = "spacePath";
/// Where the Electron app kept the one space before the folder could be
/// chosen, under its data folder. Still its space while `spacePath` is
/// unset.
const LEGACY_SPACE: &str = "workspaces/default";

/// The scratch space's folder, under this app's data folder.
const SCRATCH_SPACE: &str = "scratch-space";

/// Which space the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum SpaceChoice {
    /// `--space scratch`, and the winit shell with none named: the scratch
    /// space.
    #[default]
    Scratch,
    /// The GPUI Kit shell with none named: the folder chosen in this app,
    /// and the first-run view when there is none.
    Chosen,
    /// `--space user`: the space the Electron app opens, else the folder
    /// last chosen in this app.
    User,
    /// A folder or a `.canvas` file, from `--space PATH` or a bare path.
    Path(PathBuf),
}

/// What to open at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpaceStart {
    /// The space folder.
    pub(crate) folder: PathBuf,
    /// The canvas file to show, by name, when the launch named one.
    /// Otherwise the space's last active canvas.
    pub(crate) file: Option<String>,
    /// Whether this is the scratch space and not a folder anyone chose.
    pub(crate) scratch: bool,
}

/// Where the scratch space is kept: in this app's data folder, else, with
/// no home to put that under, the system's temp folder.
pub(crate) fn scratch_folder(app_data: Option<&Path>) -> PathBuf {
    app_data.map_or_else(
        || {
            std::env::temp_dir()
                .join("specular-native")
                .join(SCRATCH_SPACE)
        },
        |folder| folder.join(SCRATCH_SPACE),
    )
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

/// What a launch does about a space: open one, or ask the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Startup {
    /// Open this one.
    Open(SpaceStart),
    /// Open nothing and ask.
    Ask(SpaceAsk),
}

/// The space a launch opens. A path is taken as given: a folder is the
/// space, and a `.canvas` file is shown in the space its folder is. The
/// scratch space is at `scratch`. The chosen space is the folder this app
/// remembers, and `--space user` puts the Electron app's before it. Either
/// way a folder that is not there is not opened, because an unmounted
/// drive or a renamed folder is a question for the user, and making an
/// empty space in its place would answer it for them. The Electron app's
/// folder is offered in the question when it is there.
pub(crate) fn startup(
    choice: &SpaceChoice,
    electron: Option<PathBuf>,
    remembered: Option<PathBuf>,
    scratch: PathBuf,
) -> Startup {
    let offered = |electron: Option<PathBuf>| {
        (electron.filter(|folder| folder.is_dir())).map(|folder| folder.display().to_string())
    };
    let folder = match choice {
        SpaceChoice::Path(path) => return Startup::Open(from_argument(path)),
        SpaceChoice::Scratch => {
            return Startup::Open(SpaceStart {
                folder: scratch,
                file: None,
                scratch: true,
            });
        }
        SpaceChoice::User => electron.clone().or(remembered),
        SpaceChoice::Chosen => remembered,
    };
    let Some(folder) = folder else {
        tracing::info!("no space is chosen yet");
        return Startup::Ask(SpaceAsk {
            missing: None,
            electron: offered(electron),
        });
    };
    if !folder.is_dir() {
        tracing::warn!(folder = %folder.display(), "the chosen space folder is not there");
        let electron = offered(electron.filter(|electron| *electron != folder));
        return Startup::Ask(SpaceAsk {
            missing: Some(folder.display().to_string()),
            electron,
        });
    }
    Startup::Open(SpaceStart {
        folder,
        file: None,
        scratch: false,
    })
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
            scratch: false,
        };
    }
    SpaceStart {
        folder: (absolute.parent()).map_or_else(|| PathBuf::from("."), Path::to_owned),
        file: (absolute.file_name()).map(|name| name.to_string_lossy().into_owned()),
        scratch: false,
    }
}
