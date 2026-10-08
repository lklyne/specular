//! The preferences file: app settings that are not part of any canvas.
//!
//! One JSON object in the app's config folder. This shell reads and writes
//! `toolDefaults`, `spacePath`, `show` and `themeMode`, and keeps every other
//! key as it found it. The file is the
//! native app's own: the Electron app keeps its settings in memory and
//! rewrites its file whole, so two writers on one file would lose changes.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use specular_interact::{AppSettings, Theme, ToolDefaults};

use crate::persist::write_atomic;

const FILE_NAME: &str = "preferences.json";
const TOOL_DEFAULTS_KEY: &str = "toolDefaults";
const SPACE_PATH_KEY: &str = "spacePath";
/// What is shown when the app opens: `{"sidebar": bool, "rightPanel": bool}`.
const SHOW_KEY: &str = "show";
/// The key the Electron app stores its theme under too.
const THEME_KEY: &str = "themeMode";
/// Overrides the config folder, for a run that must not touch the real one.
pub(crate) const CONFIG_DIR_VARIABLE: &str = "SPECULAR_NATIVE_CONFIG_DIR";

/// Where the preferences file is, or `None` when the environment names no
/// home to put it under.
pub(crate) fn file() -> Option<PathBuf> {
    Some(folder()?.join(FILE_NAME))
}

/// This app's own data folder, which holds the preferences file and the
/// scratch space.
pub(crate) fn folder() -> Option<PathBuf> {
    config_dir(|name| std::env::var_os(name), cfg!(target_os = "macos"))
}

/// The app's config folder: the override, else Application Support on macOS,
/// else the XDG config folder.
fn config_dir(variable: impl Fn(&str) -> Option<OsString>, macos: bool) -> Option<PathBuf> {
    let set = |name: &str| variable(name).filter(|value| !value.is_empty());
    if let Some(folder) = set(CONFIG_DIR_VARIABLE) {
        return Some(PathBuf::from(folder));
    }
    let home = set("HOME").map(PathBuf::from);
    if macos {
        return Some(home?.join("Library/Application Support/Specular Native"));
    }
    let config =
        (set("XDG_CONFIG_HOME").map(PathBuf::from)).or(home.map(|home| home.join(".config")));
    Some(config?.join("specular-native"))
}

/// The file's top-level object. A missing file is an empty one.
fn read(path: &Path) -> io::Result<Map<String, Value>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(error) => return Err(error),
    };
    match serde_json::from_str(&text) {
        Ok(Value::Object(preferences)) => Ok(preferences),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "preferences are not a JSON object",
        )),
        Err(error) => Err(io::Error::new(io::ErrorKind::InvalidData, error)),
    }
}

/// The saved tool defaults, or `None` when there are none to read.
pub(crate) fn load_tool_defaults(path: &Path) -> Option<ToolDefaults> {
    match read(path) {
        Ok(preferences) => preferences
            .get(TOOL_DEFAULTS_KEY)
            .map(ToolDefaults::from_json),
        Err(error) => {
            tracing::warn!(path = %path.display(), "preferences not read: {error}");
            None
        }
    }
}

/// Writes `defaults` under `toolDefaults`, keeping the file's other keys. A
/// file that cannot be read as preferences is left alone.
pub(crate) fn save_tool_defaults(path: &Path, defaults: &ToolDefaults) -> io::Result<()> {
    save_key(path, TOOL_DEFAULTS_KEY, defaults.to_json())
}

/// The saved theme choice, or `None` when there is none to read.
pub(crate) fn load_theme(path: &Path) -> Option<Theme> {
    let preferences = read(path).ok()?;
    Some(Theme::from_key(preferences.get(THEME_KEY)?.as_str()?))
}

/// Writes `theme` under `themeMode`, keeping the file's other keys.
pub(crate) fn save_theme(path: &Path, theme: Theme) -> io::Result<()> {
    save_key(path, THEME_KEY, Value::from(theme.key()))
}

/// The space folder last chosen in this app, if one was.
pub(crate) fn load_space_path(path: &Path) -> Option<PathBuf> {
    let preferences = read(path).ok()?;
    let folder = preferences.get(SPACE_PATH_KEY)?.as_str()?;
    (!folder.is_empty()).then(|| PathBuf::from(folder))
}

/// Writes `folder` under `spacePath`, keeping the file's other keys.
pub(crate) fn save_space_path(path: &Path, folder: &Path) -> io::Result<()> {
    save_key(path, SPACE_PATH_KEY, Value::from(folder.to_string_lossy()))
}

/// The saved settings. A missing key is its default.
pub(crate) fn load_settings(path: &Path) -> AppSettings {
    let preferences = read(path).unwrap_or_default();
    let shown = |key: &str| {
        (preferences.get(SHOW_KEY))
            .and_then(|show| show.get(key))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    AppSettings {
        show_sidebar: shown("sidebar"),
        show_chat: shown("rightPanel"),
    }
}

/// Writes `settings` under `show`, keeping the file's other keys.
pub(crate) fn save_settings(path: &Path, settings: AppSettings) -> io::Result<()> {
    let show = serde_json::json!({
        "sidebar": settings.show_sidebar,
        "rightPanel": settings.show_chat,
    });
    save_key(path, SHOW_KEY, show)
}

fn save_key(path: &Path, key: &str, value: Value) -> io::Result<()> {
    let mut preferences = read(path)?;
    preferences.insert(key.to_owned(), value);
    let text = serde_json::to_string_pretty(&Value::Object(preferences))?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    write_atomic(path, &format!("{text}\n"))
}

#[cfg(test)]
mod tests {
    use specular_doc::ShapeKind;
    use specular_interact::ToolDefaultPatch;

    use super::*;

    /// A fresh folder under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("specular-prefs-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn diamonds() -> ToolDefaults {
        let mut defaults = ToolDefaults::default();
        defaults.apply(ToolDefaultPatch::ShapeKind(ShapeKind::Diamond));
        defaults
    }

    fn environment(
        pairs: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Option<OsString> {
        move |name| {
            (pairs.iter())
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn the_config_folder_follows_the_platform() {
        let home = environment(&[("HOME", "/Users/me")]);
        assert_eq!(
            config_dir(&home, true),
            Some(PathBuf::from(
                "/Users/me/Library/Application Support/Specular Native"
            ))
        );
        assert_eq!(
            config_dir(&home, false),
            Some(PathBuf::from("/Users/me/.config/specular-native"))
        );
        let xdg = environment(&[("HOME", "/home/me"), ("XDG_CONFIG_HOME", "/cfg")]);
        assert_eq!(
            config_dir(xdg, false),
            Some(PathBuf::from("/cfg/specular-native"))
        );

        let overridden = environment(&[("HOME", "/Users/me"), (CONFIG_DIR_VARIABLE, "/tmp/cfg")]);
        assert_eq!(
            config_dir(overridden, true),
            Some(PathBuf::from("/tmp/cfg"))
        );
        assert_eq!(config_dir(environment(&[]), true), None);
        assert_eq!(config_dir(environment(&[("HOME", "")]), false), None);
    }

    #[test]
    fn saved_defaults_are_read_back() {
        let dir = TempDir::new("round-trip");
        let path = dir.0.join("nested").join(FILE_NAME);
        assert_eq!(load_tool_defaults(&path), None);
        save_tool_defaults(&path, &diamonds()).unwrap();
        assert_eq!(load_tool_defaults(&path), Some(diamonds()));
        assert_eq!(load_theme(&path), None);
        save_theme(&path, Theme::Dark).unwrap();
        assert_eq!(load_theme(&path), Some(Theme::Dark));
        assert_eq!(load_tool_defaults(&path), Some(diamonds()));
    }

    #[test]
    fn a_save_keeps_the_other_preferences() {
        let dir = TempDir::new("other-keys");
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join(FILE_NAME);
        std::fs::write(
            &path,
            r#"{"spacePath":"/Users/me/Space","themeMode":"dark","toolDefaults":{"draw":{}}}"#,
        )
        .unwrap();
        save_tool_defaults(&path, &diamonds()).unwrap();
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["spacePath"], "/Users/me/Space");
        assert_eq!(saved["themeMode"], "dark");
        assert_eq!(saved["toolDefaults"]["add-shape"]["shapeKind"], "diamond");
    }

    #[test]
    fn saved_settings_are_read_back_beside_the_space_folder() {
        let dir = TempDir::new("settings");
        let path = dir.0.join(FILE_NAME);
        assert_eq!(load_settings(&path), AppSettings::default());
        save_space_path(&path, Path::new("/Users/me/Space")).unwrap();
        let settings = AppSettings {
            show_sidebar: true,
            show_chat: false,
        };
        save_settings(&path, settings).unwrap();
        assert_eq!(load_settings(&path), settings);
        assert_eq!(
            load_space_path(&path),
            Some(PathBuf::from("/Users/me/Space"))
        );
    }

    #[test]
    fn a_file_that_is_not_preferences_is_left_alone() {
        let dir = TempDir::new("damaged");
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join(FILE_NAME);
        std::fs::write(&path, "[1, 2").unwrap();
        assert_eq!(load_tool_defaults(&path), None);
        assert!(save_tool_defaults(&path, &diamonds()).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[1, 2");
    }
}
