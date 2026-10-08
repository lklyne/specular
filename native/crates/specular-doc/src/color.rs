//! Canvas colors: a JSON Canvas preset, the theme-aware neutral, or a
//! literal the renderer passes through.

use serde::{Deserialize, Serialize};

/// The six JSON Canvas preset colors, stored as `"1"` to `"6"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorPreset {
    /// Preset `"1"`.
    Red,
    /// Preset `"2"`.
    Orange,
    /// Preset `"3"`.
    Yellow,
    /// Preset `"4"`.
    Green,
    /// Preset `"5"`.
    Cyan,
    /// Preset `"6"`.
    Purple,
}

impl ColorPreset {
    /// The preset's stored string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Red => "1",
            Self::Orange => "2",
            Self::Yellow => "3",
            Self::Green => "4",
            Self::Cyan => "5",
            Self::Purple => "6",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "1" => Self::Red,
            "2" => Self::Orange,
            "3" => Self::Yellow,
            "4" => Self::Green,
            "5" => Self::Cyan,
            "6" => Self::Purple,
            _ => return None,
        })
    }
}

/// A stored color. Serializes as its string form.
///
/// On a node, the neutral is written as the red preset plus
/// `specular.colorRole: "neutral"` so other tools still see a valid color;
/// the `.canvas` writer owns that mapping. Everywhere else it is the string
/// `"neutral"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Color {
    /// Resolved from the active theme and the entity's color role.
    Neutral,
    /// A JSON Canvas preset.
    Preset(ColorPreset),
    /// Any other string, normally `#RRGGBB`.
    Custom(String),
}

impl Color {
    const NEUTRAL: &'static str = "neutral";

    /// Reads a stored color string.
    pub fn parse(value: &str) -> Self {
        if value == Self::NEUTRAL {
            return Self::Neutral;
        }
        ColorPreset::parse(value).map_or_else(|| Self::Custom(value.to_owned()), Self::Preset)
    }

    /// The stored string form.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Neutral => Self::NEUTRAL,
            Self::Preset(preset) => preset.as_str(),
            Self::Custom(value) => value,
        }
    }
}

impl From<String> for Color {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<Color> for String {
    fn from(color: Color) -> Self {
        match color {
            Color::Custom(value) => value,
            Color::Neutral | Color::Preset(_) => color.as_str().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_strings_round_trip() {
        for stored in ["neutral", "1", "6", "#ff00aa", "7"] {
            assert_eq!(Color::parse(stored).as_str(), stored);
        }

        {
            assert_eq!(Color::parse("3"), Color::Preset(ColorPreset::Yellow));
            assert_eq!(Color::parse("neutral"), Color::Neutral);
        }
    }
}
