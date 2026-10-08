//! The tool defaults', the settings' and the theme's trip to and from the
//! preferences file, and the theme's other effect: the colour scheme pages
//! report.

use specular_core::PageColorScheme;
use specular_doc::{ColorScheme, Document, EntityId, Kind};
use specular_interact::{AppSettings, Appearance, Event, Theme, ToolDefaults};

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

    /// Hands the app the theme saved by an earlier run.
    pub(super) fn load_theme(&mut self) {
        if let Some(theme) = self.prefs.as_deref().and_then(prefs::load_theme) {
            self.dispatch(Event::ThemeLoaded(theme));
        }
    }

    pub(super) fn save_theme(&self, theme: Theme) {
        let Some(path) = self.prefs.as_deref() else {
            return;
        };
        if let Err(error) = prefs::save_theme(path, theme) {
            tracing::warn!(path = %path.display(), "theme not saved: {error}");
        }
    }

    /// Tells a page just made the scheme it reports.
    pub(super) fn give_color_scheme(&mut self, page: &EntityId) {
        let own = page_own_scheme(self.app.document(), page);
        self.set_page_color_scheme(page, own);
    }

    /// Tells a hosted page the scheme it reports: its own, or the app's.
    pub(super) fn set_page_color_scheme(&mut self, page: &EntityId, scheme: Option<ColorScheme>) {
        if let Some(host) = self.hosts.get(page).map(|host| host.page) {
            let scheme = page_color_scheme(scheme, self.app.appearance());
            if let Err(error) = self.source.set_color_scheme(host, scheme) {
                tracing::warn!(%page, "color scheme not set: {error}");
            }
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

/// The scheme a page reports when its own setting is `own` and the app is
/// drawn in `appearance`.
pub(crate) fn page_color_scheme(
    own: Option<ColorScheme>,
    appearance: Appearance,
) -> PageColorScheme {
    match (own, appearance) {
        (Some(ColorScheme::Light), _) | (None, Appearance::Light) => PageColorScheme::Light,
        (Some(ColorScheme::Dark), _) | (None, Appearance::Dark) => PageColorScheme::Dark,
    }
}

/// The scheme the page `page` of `document` is set to, if it is set to one.
pub(crate) fn page_own_scheme(document: &Document, page: &EntityId) -> Option<ColorScheme> {
    match &document.entity(page)?.kind {
        Kind::Page(page) => page.color_scheme,
        Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
    }
}
