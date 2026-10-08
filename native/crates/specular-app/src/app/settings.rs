//! The tool defaults' and the settings' trip to and from the preferences
//! file.

use specular_interact::{AppSettings, Event, ToolDefaults};

use super::runtime::{Runtime, ShellWindow};
use crate::prefs;

impl<W: ShellWindow> Runtime<W> {
    /// Hands the app the tool defaults saved by an earlier run.
    pub(super) fn load_tool_defaults(&mut self) {
        let saved = self.prefs.as_deref().and_then(prefs::load_tool_defaults);
        if let Some(defaults) = saved {
            self.dispatch(Event::ToolDefaultsLoaded(Box::new(defaults)));
        }
    }

    pub(super) fn save_tool_defaults(&self, defaults: &ToolDefaults) {
        let Some(path) = self.prefs.as_deref() else {
            return;
        };
        if let Err(error) = prefs::save_tool_defaults(path, defaults) {
            tracing::warn!(path = %path.display(), "tool defaults not saved: {error}");
        }
    }

    /// Hands the app the settings saved by an earlier run.
    pub(super) fn load_settings(&mut self) {
        if let Some(path) = self.prefs.as_deref() {
            let settings = prefs::load_settings(path);
            self.dispatch(Event::SettingsLoaded(settings));
        }
    }

    pub(super) fn save_settings(&self, settings: AppSettings) {
        let Some(path) = self.prefs.as_deref() else {
            return;
        };
        if let Err(error) = prefs::save_settings(path, settings) {
            tracing::warn!(path = %path.display(), "settings not saved: {error}");
        }
    }
}
