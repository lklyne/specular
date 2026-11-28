//! Where the tools are drawn: the place the setting chose, or the column
//! docked tools float in while the sidebar is hidden, and the ribbon's
//! measures both the window and a headless run's chrome guard read.

use std::ops::Range;

use crate::{App, ToolsSetting};

/// The side of a tool's button, which is square.
pub const TOOL_BUTTON: f32 = 28.0;
/// From the canvas's edge to a floating ribbon.
pub const RIBBON_INSET: f32 = 8.0;
/// From a ribbon's own edge to its buttons.
pub const RIBBON_PAD: f32 = 4.0;
/// From a tool's button to its options.
pub const OPTIONS_GAP: f32 = 10.0;
/// The hairline rim either side of a floating ribbon.
const RIBBON_RIM: f32 = 1.0;
/// Between two things in a ribbon, at the layout's default spacing.
const RIBBON_GAP: f32 = 4.0;
/// A rule between two groups of a ribbon, with its margin either side.
const RIBBON_RULE: f32 = 9.0;

/// Where the tools' ribbon is drawn now ([`App::tools_place`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsPlace {
    /// Down the canvas's left edge, taking that room from it.
    Docked,
    /// Over the top of the canvas.
    FloatTop,
    /// Over the bottom of the canvas.
    FloatBottom,
    /// A column over the canvas's left edge.
    FloatLeft,
}

/// A place is written by its name.
impl serde::Serialize for ToolsPlace {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl ToolsPlace {
    /// What the toolbar's model and the scripts' facts call it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Docked => "docked",
            Self::FloatTop => "top",
            Self::FloatBottom => "bottom",
            Self::FloatLeft => "left",
        }
    }

    /// Whether the tools run down a column rather than across a row.
    pub const fn upright(self) -> bool {
        match self {
            Self::Docked | Self::FloatLeft => true,
            Self::FloatTop | Self::FloatBottom => false,
        }
    }
}

impl From<ToolsSetting> for ToolsPlace {
    fn from(setting: ToolsSetting) -> Self {
        match setting {
            ToolsSetting::Docked => Self::Docked,
            ToolsSetting::FloatTop => Self::FloatTop,
            ToolsSetting::FloatBottom => Self::FloatBottom,
        }
    }
}

/// The span, from the window's left edge, of the column the tools float
/// in down the canvas's left edge, which starts at `canvas_left`.
pub fn float_left_column(canvas_left: f32) -> Range<f32> {
    let left = canvas_left + RIBBON_INSET;
    left..left + TOOL_BUTTON + 2.0 * (RIBBON_PAD + RIBBON_RIM)
}

/// The span, down the window, of the ribbon floating down the canvas's
/// left edge, centred between `top` and `bottom`: `items` square buttons
/// with `rules` between their groups. A layout's spacing or a toolbar mod
/// draws it a little longer, so the span reaches a button further at each
/// end.
pub fn float_left_span(top: f32, bottom: f32, items: usize, rules: usize) -> Range<f32> {
    let parts = items + rules;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a ribbon holds a few dozen parts"
    )]
    let length = 2.0 * (RIBBON_PAD + RIBBON_RIM)
        + items as f32 * TOOL_BUTTON
        + rules as f32 * RIBBON_RULE
        + parts.saturating_sub(1) as f32 * RIBBON_GAP;
    let reach = length / 2.0 + TOOL_BUTTON + RIBBON_GAP;
    let middle = top.midpoint(bottom);
    (middle - reach).max(top)..(middle + reach).min(bottom)
}

impl App {
    /// Where the tools are now: the setting, except that tools docked
    /// beside a sidebar the layout has but hides float down the canvas's
    /// left edge, and the canvas takes their column.
    pub fn tools_place(&self) -> ToolsPlace {
        match self.settings.tools {
            ToolsSetting::Docked if !self.session.sidebar.shown() && self.regions().sidebar => {
                ToolsPlace::FloatLeft
            }
            setting => setting.into(),
        }
    }
}
