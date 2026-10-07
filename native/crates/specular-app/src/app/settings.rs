//! The tool defaults' trip to and from the preferences file.

use specular_interact::{Event, ToolDefaults};

use super::Shell;
use crate::prefs;

impl Shell {
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
}
