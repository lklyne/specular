//! [`ToolDefaults`]: what each creation tool stamps on the next entity it
//! makes (ADR 0008, ADR 0009).
//!
//! These are app settings. They are not in the document, not in a `.canvas`
//! file and not in undo. The shell reads them from the preferences file at
//! startup and writes them back when
//! [`Effect::SaveToolDefaults`](crate::Effect::SaveToolDefaults) asks.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};
use specular_doc::{BrushType, Color, ColorPreset, ShapeKind, TextFont};

use crate::Tool;

/// The defaults of every tool that has some.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolDefaults {
    /// Plain text.
    pub(crate) text: TextDefaults,
    /// Sticky notes.
    pub sticky: StickyDefaults,
    /// Shapes.
    pub shape: ShapeDefaults,
    /// Freehand strokes.
    pub draw: DrawDefaults,
    /// Pages. Not stored: like Electron's, the size the next page is made
    /// at lasts as long as the app runs.
    pub(crate) page: PageDefaults,
}

/// What `add-page` makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageDefaults {
    /// The viewport preset, by index into
    /// [`VIEWPORT_PRESETS`](specular_doc::VIEWPORT_PRESETS).
    pub(crate) preset: u32,
    /// Whether the page keeps that size as a custom size rather than as the
    /// preset's.
    pub(crate) custom: bool,
}

/// What `add-text` stamps.
#[derive(Debug, Clone, PartialEq)]
pub struct TextDefaults {
    /// The ink. `None` follows the theme's foreground.
    pub(crate) color: Option<Color>,
    /// Text size in pixels.
    pub(crate) size: f64,
    /// Typeface.
    pub(crate) font: TextFont,
}

/// What `add-sticky` stamps.
#[derive(Debug, Clone, PartialEq)]
pub struct StickyDefaults {
    /// The card's fill.
    pub(crate) color: Color,
    /// Text size in pixels.
    pub(crate) size: f64,
    /// Typeface.
    pub font: TextFont,
}

/// What `add-shape` stamps.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDefaults {
    /// Which silhouette.
    pub kind: ShapeKind,
    /// Fill color.
    pub(crate) color: Color,
    /// Border width in pixels.
    pub(crate) stroke_width: f64,
    /// Label size in pixels.
    pub(crate) text_size: f64,
}

/// What `draw` stamps on each stroke.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawDefaults {
    /// Pen or highlighter.
    pub brush: BrushType,
    /// Stroke color.
    pub(crate) color: Color,
    /// Stroke width in canvas units.
    pub(crate) stroke_width: f64,
}

const DEFAULT_TEXT_SIZE: f64 = 14.0;
const DEFAULT_STROKE_WIDTH: f64 = 2.0;

impl Default for TextDefaults {
    fn default() -> Self {
        Self {
            color: None,
            size: DEFAULT_TEXT_SIZE,
            font: TextFont::Sans,
        }
    }
}

impl Default for StickyDefaults {
    fn default() -> Self {
        Self {
            color: Color::Preset(ColorPreset::Yellow),
            size: DEFAULT_TEXT_SIZE,
            font: TextFont::Sans,
        }
    }
}

impl Default for ShapeDefaults {
    fn default() -> Self {
        Self {
            kind: ShapeKind::Rectangle,
            color: Color::Preset(ColorPreset::Red),
            stroke_width: DEFAULT_STROKE_WIDTH,
            text_size: DEFAULT_TEXT_SIZE,
        }
    }
}

impl Default for DrawDefaults {
    fn default() -> Self {
        Self {
            brush: BrushType::Pen,
            color: Color::Preset(ColorPreset::Red),
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }
}

/// A change to one tool default.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolDefaultPatch {
    /// Plain text ink. `None` follows the theme.
    TextColor(Option<Color>),
    /// Plain text size.
    TextSize(f64),
    /// Plain text typeface.
    TextFont(TextFont),
    /// Sticky fill.
    StickyColor(Color),
    /// Sticky text size.
    StickySize(f64),
    /// Sticky typeface.
    StickyFont(TextFont),
    /// Which shape `add-shape` places.
    ShapeKind(ShapeKind),
    /// Shape fill.
    ShapeColor(Color),
    /// Shape border width.
    ShapeStrokeWidth(f64),
    /// Shape label size.
    ShapeTextSize(f64),
    /// Which brush `draw` uses.
    Brush(BrushType),
    /// Stroke color.
    DrawColor(Color),
    /// Stroke width.
    DrawStrokeWidth(f64),
    /// Which viewport preset `add-page` makes a page at.
    PagePreset(u32),
    /// `add-page` makes a page of custom size.
    PageCustom,
}

impl ToolDefaultPatch {
    /// The tool whose default this changes.
    pub(crate) const fn tool(&self) -> Tool {
        match self {
            Self::TextColor(_) | Self::TextSize(_) | Self::TextFont(_) => Tool::AddText,
            Self::StickyColor(_) | Self::StickySize(_) | Self::StickyFont(_) => Tool::AddSticky,
            Self::ShapeKind(_)
            | Self::ShapeColor(_)
            | Self::ShapeStrokeWidth(_)
            | Self::ShapeTextSize(_) => Tool::AddShape,
            Self::Brush(_) | Self::DrawColor(_) | Self::DrawStrokeWidth(_) => Tool::Draw,
            Self::PagePreset(_) | Self::PageCustom => Tool::AddPage,
        }
    }

    /// Whether the preferences file keeps what this changes.
    pub(crate) const fn is_stored(&self) -> bool {
        match self {
            Self::PagePreset(_) | Self::PageCustom => false,
            Self::TextColor(_)
            | Self::TextSize(_)
            | Self::TextFont(_)
            | Self::StickyColor(_)
            | Self::StickySize(_)
            | Self::StickyFont(_)
            | Self::ShapeKind(_)
            | Self::ShapeColor(_)
            | Self::ShapeStrokeWidth(_)
            | Self::ShapeTextSize(_)
            | Self::Brush(_)
            | Self::DrawColor(_)
            | Self::DrawStrokeWidth(_) => true,
        }
    }
}

/// The stroke widths a brush is offered in, thin to thick. A highlighter
/// is a marker: far wider than a pen.
pub(crate) fn width_presets(brush: BrushType) -> [f64; 2] {
    match brush {
        BrushType::Pen => [2.0, 4.0],
        BrushType::Highlight => [8.0, 16.0],
    }
}

/// The width `brush` is offered in that is closest to `width`.
pub(crate) fn nearest_width(brush: BrushType, width: f64) -> f64 {
    let [thin, thick] = width_presets(brush);
    if (width - thin).abs() <= (width - thick).abs() {
        thin
    } else {
        thick
    }
}

impl ToolDefaults {
    /// Applies one change. Changing the brush also moves the stroke width
    /// to the nearest width that brush is offered in.
    pub(crate) fn apply(&mut self, patch: ToolDefaultPatch) {
        match patch {
            ToolDefaultPatch::TextColor(color) => self.text.color = color,
            ToolDefaultPatch::TextSize(size) => self.text.size = size,
            ToolDefaultPatch::TextFont(font) => self.text.font = font,
            ToolDefaultPatch::StickyColor(color) => self.sticky.color = color,
            ToolDefaultPatch::StickySize(size) => self.sticky.size = size,
            ToolDefaultPatch::StickyFont(font) => self.sticky.font = font,
            ToolDefaultPatch::ShapeKind(kind) => self.shape.kind = kind,
            ToolDefaultPatch::ShapeColor(color) => self.shape.color = color,
            ToolDefaultPatch::ShapeStrokeWidth(width) => self.shape.stroke_width = width,
            ToolDefaultPatch::ShapeTextSize(size) => self.shape.text_size = size,
            ToolDefaultPatch::Brush(brush) => {
                self.draw.brush = brush;
                self.draw.stroke_width = nearest_width(brush, self.draw.stroke_width);
            }
            ToolDefaultPatch::DrawColor(color) => self.draw.color = color,
            ToolDefaultPatch::DrawStrokeWidth(width) => self.draw.stroke_width = width,
            ToolDefaultPatch::PagePreset(preset) => {
                self.page = PageDefaults {
                    preset,
                    custom: false,
                }
            }
            ToolDefaultPatch::PageCustom => self.page.custom = true,
        }
    }

    /// Reads the `toolDefaults` value of the preferences file. Anything
    /// missing or of the wrong type keeps its default, so a file from an
    /// older version, or a damaged one, still loads.
    pub fn from_json(raw: &Value) -> Self {
        let mut defaults = Self::default();
        let scope = |key: &str| raw.get(key).and_then(Value::as_object);
        let text = scope("add-text");
        if let Some(text) = text {
            // Before the sticky had a tool of its own, both colors sat under
            // `add-text`. The dotted key wins, as it does in Electron.
            for key in ["color", "plain.color"] {
                match text.get(key) {
                    Some(Value::Null) => defaults.text.color = None,
                    Some(Value::String(color)) => defaults.text.color = Some(Color::parse(color)),
                    Some(_) | None => {}
                }
            }
            set(&mut defaults.text.size, text, "textSize");
            set(&mut defaults.text.font, text, "textFont");
        }
        if let Some(sticky) = scope("add-sticky") {
            set(&mut defaults.sticky.color, sticky, "color");
            set(&mut defaults.sticky.size, sticky, "textSize");
            set(&mut defaults.sticky.font, sticky, "textFont");
        } else if let Some(text) = text {
            set(&mut defaults.sticky.color, text, "sticky.color");
        }
        if let Some(shape) = scope("add-shape") {
            set(&mut defaults.shape.kind, shape, "shapeKind");
            set(&mut defaults.shape.color, shape, "color");
            set(&mut defaults.shape.stroke_width, shape, "strokeWidth");
            set(&mut defaults.shape.text_size, shape, "textSize");
        }
        if let Some(draw) = scope("draw") {
            set(&mut defaults.draw.brush, draw, "brushType");
            set(&mut defaults.draw.color, draw, "color");
            set(&mut defaults.draw.stroke_width, draw, "strokeWidth");
        }
        defaults
    }

    /// The value to store under `toolDefaults` in the preferences file, in
    /// the shape Electron writes.
    pub fn to_json(&self) -> Value {
        json!({
            "add-text": {
                "color": self.text.color.as_ref().map(Color::as_str),
                "textSize": self.text.size,
                "textFont": stored(&self.text.font),
            },
            "add-sticky": {
                "color": self.sticky.color.as_str(),
                "textSize": self.sticky.size,
                "textFont": stored(&self.sticky.font),
            },
            "add-shape": {
                "shapeKind": stored(&self.shape.kind),
                "color": self.shape.color.as_str(),
                "strokeWidth": self.shape.stroke_width,
                "textSize": self.shape.text_size,
            },
            "draw": {
                "brushType": stored(&self.draw.brush),
                "color": self.draw.color.as_str(),
                "strokeWidth": self.draw.stroke_width,
            },
        })
    }
}

/// Overwrites `slot` with `scope[key]` when that reads as a `T`.
fn set<T: DeserializeOwned>(slot: &mut T, scope: &Map<String, Value>, key: &str) {
    if let Some(value) = scope.get(key)
        && let Ok(value) = T::deserialize(value)
    {
        *slot = value;
    }
}

/// A value enum as its stored string.
fn stored<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_malformed_values_keep_their_defaults() {
        for raw in [json!(null), json!("nope"), json!({ "draw": 4 })] {
            assert_eq!(ToolDefaults::from_json(&raw), ToolDefaults::default());
        }
        let partial = ToolDefaults::from_json(&json!({
            "add-shape": { "shapeKind": "blob", "strokeWidth": "thick", "color": "5" },
            "draw": { "brushType": "highlight" },
        }));
        assert_eq!(partial.shape.kind, ShapeKind::Rectangle);
        assert_eq!(partial.shape.stroke_width, 2.0);
        assert_eq!(partial.shape.color, Color::Preset(ColorPreset::Cyan));
        assert_eq!(partial.draw.brush, BrushType::Highlight);
        assert_eq!(partial.draw.stroke_width, 2.0);
    }
}
