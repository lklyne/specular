//! A layout's look (ADR 0059): its theme, colours and fonts, set on the
//! layout's root and read by the app's own panels and by every mod.
//!
//! ```json
//! { "theme": { "light": "paper", "dark": "graphite" },
//!   "colors": { "sidebar": "#101418" }, "font": "Inter", "font_size": 13 }
//! ```
//!
//! A theme is `light`, `dark`, or the name of a file in `themes/`
//! (`{"appearance": "dark", "colors": {..}}`), which the shell reads and
//! hands the app. `colors` overrides any colour by name, over the theme's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::node::Border;
use crate::Appearance;

/// The theme a layout names: one, or one for each appearance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeChoice {
    /// The same theme in light and dark.
    One(String),
    /// A theme for each appearance the system is in.
    Each {
        /// In light mode.
        light: String,
        /// In dark mode.
        dark: String,
    },
}

/// A theme file, `themes/<name>.json`: everything that styles the app's
/// own panels, menus and popups and every mod, set once. A layout's own
/// keys of the same names go over it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeFile {
    /// Whether it is a light or a dark theme.
    #[serde(default)]
    pub appearance: Option<Appearance>,
    /// Its colours, by name.
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
    /// The chrome's font family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// The chrome's text size, in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    /// The corner radius of the app's controls, menus, popups and dialogs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
}

/// What a layout's root sets of the look.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// The theme, else the person's choice in the settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemeChoice>,
    /// Colours by name, over the theme's.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, String>,
    /// The chrome's font family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// The chrome's text size, in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    /// The corner radius of the app's own controls and popups, in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
    /// Painted behind the whole window where no block paints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Value>,
    /// A line round the whole window, over every block, rounded as macOS
    /// rounds the window's corners: `{"width": 1, "color": "#000"}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_border: Option<Border>,
    /// Where macOS draws the window's close, minimize and zoom buttons.
    #[serde(default, skip_serializing_if = "TrafficLights::is_native")]
    pub traffic_lights: TrafficLights,
}

/// Where the window's close, minimize and zoom buttons are: where the app
/// puts them, `"hidden"` for a layout that draws its own (with the
/// `window.close`, `app.minimize` and `window.zoom` commands), or the
/// first button's corner `{ "x": 14, "y": 13 }` in points from the
/// window's top-left.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TrafficLights {
    /// Where the app puts them.
    #[default]
    #[serde(skip)]
    Native,
    /// Not drawn.
    Hidden(HiddenWord),
    /// The first button's corner.
    At {
        /// Points from the window's left edge.
        x: f32,
        /// Points from the window's top edge.
        y: f32,
    },
}

/// The word `"hidden"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HiddenWord {
    /// `"hidden"`.
    Hidden,
}

impl TrafficLights {
    /// Whether the app puts them where it always does.
    pub const fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }
}

/// The look in force: the appearance drawn, the theme behind it, and every
/// colour that differs from the app's own for that appearance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resolved {
    /// The theme's name, when one is in force.
    pub theme: Option<String>,
    /// What the theme says the appearance is, when it says.
    pub appearance: Option<Appearance>,
    /// Colours by name, the theme's then the layout's.
    pub colors: BTreeMap<String, String>,
    /// The chrome's font family: the layout's, else the theme's.
    pub font: Option<String>,
    /// The chrome's text size: the layout's, else the theme's.
    pub font_size: Option<f32>,
    /// The corner radius: the layout's, else the theme's.
    pub radius: Option<f32>,
}

impl Look {
    /// The look with `system` the appearance asked for and `themes` the
    /// theme files read, by name. A theme named `light` or `dark` with no
    /// file is that appearance with the app's own colours.
    pub fn resolve(&self, system: Appearance, themes: &BTreeMap<String, ThemeFile>) -> Resolved {
        let name = self.theme_name(system);
        let file = name.and_then(|name| themes.get(name));
        let mut colors = file.map_or_else(Default::default, |file| file.colors.clone());
        colors.extend(self.colors.clone());
        Resolved {
            theme: name.map(str::to_owned),
            appearance: self.appearance(system, themes),
            colors,
            font: (self.font.clone()).or_else(|| file.and_then(|file| file.font.clone())),
            font_size: self
                .font_size
                .or_else(|| file.and_then(|file| file.font_size)),
            radius: self.radius.or_else(|| file.and_then(|file| file.radius)),
        }
    }

    /// The theme in force when the system asks for `system`.
    fn theme_name(&self, system: Appearance) -> Option<&str> {
        self.theme.as_ref().map(|choice| match choice {
            ThemeChoice::One(name) => name.as_str(),
            ThemeChoice::Each { light, dark } => match system {
                Appearance::Light => light,
                Appearance::Dark => dark,
            },
        })
    }

    /// The appearance the theme in force pins, if it pins one.
    pub fn appearance(
        &self,
        system: Appearance,
        themes: &BTreeMap<String, ThemeFile>,
    ) -> Option<Appearance> {
        let name = self.theme_name(system)?;
        (themes.get(name).and_then(|file| file.appearance)).or(match name {
            "light" => Some(Appearance::Light),
            "dark" => Some(Appearance::Dark),
            _ => None,
        })
    }

    /// The look as the `look` state key holds it.
    pub fn to_json(&self, resolved: &Resolved, appearance: Appearance) -> Value {
        let colors: Map<String, Value> = (resolved.colors.iter())
            .map(|(name, value)| (name.clone(), Value::String(value.clone())))
            .collect();
        json!({
            "appearance": match appearance {
                Appearance::Light => "light",
                Appearance::Dark => "dark",
            },
            "theme": resolved.theme,
            "colors": colors,
            "font": resolved.font,
            "fontSize": resolved.font_size,
            "radius": resolved.radius,
            "background": self.background,
        })
    }
}
