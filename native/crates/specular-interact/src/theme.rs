//! [`Theme`]: the appearance preference, and the [`Appearance`] it resolves to.
//!
//! The preference is an app setting like the tool defaults: not in the
//! document, not in undo, read from the preferences file at startup and
//! written back by [`Effect::SaveTheme`](crate::Effect::SaveTheme). `System`
//! follows the operating system, whose appearance the shell reports with
//! [`Event::SystemAppearance`](crate::Event::SystemAppearance); this crate
//! does no I/O.

use crate::{App, Effect};

/// Light or dark: what is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Appearance {
    /// The light theme.
    #[default]
    Light,
    /// The dark theme.
    Dark,
}

/// What the user chose: a fixed appearance, or the system's
/// (`AppThemeMode` in `src/shared/types.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Theme {
    /// Follow the operating system.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl Theme {
    /// The choice the toolbar button moves to: system, light, dark, round
    /// again (`nextThemeMode`).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }

    /// The word for the choice: `System`, `Light` or `Dark`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    /// The name stored in the preferences file's `themeMode`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// The choice a stored `themeMode` names. Anything else is `System`, as
    /// `normalizeThemeMode` has it.
    #[must_use]
    pub fn from_key(key: &str) -> Self {
        match key {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    /// What is drawn when the system is `system`.
    #[must_use]
    pub const fn resolve(self, system: Appearance) -> Appearance {
        match self {
            Self::System => system,
            Self::Light => Appearance::Light,
            Self::Dark => Appearance::Dark,
        }
    }
}

/// The choice and what the system last said, which together say what to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThemeState {
    pub(crate) choice: Theme,
    pub(crate) system: Appearance,
}

impl ThemeState {
    /// The user's choice.
    #[must_use]
    pub const fn choice(self) -> Theme {
        self.choice
    }

    /// What is drawn now.
    #[must_use]
    pub const fn appearance(self) -> Appearance {
        self.choice.resolve(self.system)
    }
}

/// Makes `theme` the choice. A user's pick (`save`) is written to the
/// preferences; one read from them is not. Pages are told the appearance
/// they now follow either way.
pub(crate) fn choose(app: &mut App, theme: Theme, save: bool, effects: &mut Vec<Effect>) {
    let before = app.theme;
    app.theme.choice = theme;
    if save && before.choice != theme {
        effects.push(Effect::SaveTheme(theme));
    }
    if before.appearance() != app.theme.appearance() {
        crate::pages::refresh_color_schemes(&app.document, effects);
    }
}

/// The system's appearance is `system`.
pub(crate) fn system_changed(app: &mut App, system: Appearance, effects: &mut Vec<Effect>) {
    let before = app.theme.appearance();
    app.theme.system = system;
    if before != app.theme.appearance() {
        crate::pages::refresh_color_schemes(&app.document, effects);
    }
}
