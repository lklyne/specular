//! The per-kind payloads of an [`Entity`](crate::Entity).
//!
//! Optional fields stay `Option` when the format distinguishes "absent" from
//! a value, so a file that never set a field saves without it. Readers apply
//! the default through the `resolved_*` accessors.

use serde::{Deserialize, Serialize};

use crate::{Color, JsonMap, Point};

/// A live web page (a `link` node).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Page {
    /// Full URL, scheme and host included.
    pub url: String,
    /// Index into the viewport preset table.
    pub preset_index: Option<u32>,
    /// Sync set membership: pages sharing an id navigate and scroll together.
    pub sync_id: Option<String>,
    /// Whether the user or an agent created the page.
    pub source: Option<PageSource>,
    /// Color-scheme override. Absent follows the system.
    pub color_scheme: Option<ColorScheme>,
    /// Device and sizing metadata (`pageSizeMode`, `customSize`, ...).
    pub metadata: Option<JsonMap>,
}

impl Page {
    /// The device frame the page is drawn in, or `None` when its
    /// `showDeviceFrame` is off. The frame sits outside the entity's rect,
    /// which is the page's screen.
    pub fn shell(&self) -> Option<crate::DeviceShell> {
        let meta = self.metadata.as_ref()?;
        if meta.get("showDeviceFrame") != Some(&serde_json::Value::Bool(true)) {
            return None;
        }
        let device = meta.get("deviceId").and_then(serde_json::Value::as_str);
        let landscape = meta
            .get("deviceOrientation")
            .and_then(serde_json::Value::as_str)
            == Some("landscape");
        Some(crate::device_shell(device, landscape))
    }
}

/// Who created a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageSource {
    /// Created by the user.
    Manual,
    /// Created by an agent.
    Generated,
}

/// A page's color-scheme override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    /// Force light.
    Light,
    /// Force dark.
    Dark,
}

/// Plain text or a sticky note.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Text {
    /// The content.
    pub text: String,
    /// Sticky fill or plain text color.
    pub color: Option<Color>,
    /// Plain or sticky. See [`resolved_style`](Self::resolved_style).
    pub style: Option<TextStyle>,
    /// Auto or fixed width. See [`resolved_width_mode`](Self::resolved_width_mode).
    pub width_mode: Option<WidthMode>,
    /// Text size in pixels. Absent renders at the default size.
    pub size: Option<f64>,
    /// Typeface token. Absent renders in sans.
    pub font: Option<TextFont>,
}

impl Text {
    /// The style, defaulting to sticky as legacy canvases expect.
    pub fn resolved_style(&self) -> TextStyle {
        self.style.unwrap_or(TextStyle::Sticky)
    }

    /// The width mode: plain text grows with its content, stickies are fixed.
    pub fn resolved_width_mode(&self) -> WidthMode {
        self.width_mode.unwrap_or(match self.resolved_style() {
            TextStyle::Plain => WidthMode::Auto,
            TextStyle::Sticky => WidthMode::Fixed,
        })
    }
}

/// How a text entity is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextStyle {
    /// Unbacked text.
    Plain,
    /// A colored card.
    Sticky,
}

/// How a text entity sizes itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WidthMode {
    /// Width follows the content.
    Auto,
    /// Width is set by the user; content wraps.
    Fixed,
}

/// A semantic typeface token. The renderer resolves it to a family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextFont {
    /// The UI sans face.
    Sans,
    /// The monospace face.
    Mono,
    /// The handwriting face.
    Hand,
}

/// A file from the space folder shown on the canvas (image, markdown, ...).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileRef {
    /// Path relative to the space folder.
    pub file: String,
    /// A heading or block inside the file, starting with `#`.
    pub subpath: Option<String>,
    /// How media fills the entity rect.
    pub object_fit: Option<ObjectFit>,
    /// Index into the viewport preset table, for files shown at a breakpoint.
    pub preset_index: Option<u32>,
    /// Renderer metadata.
    pub metadata: Option<JsonMap>,
}

/// How media fills its rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectFit {
    /// Letterbox inside the rect.
    Contain,
    /// Crop to cover the rect.
    Cover,
    /// Stretch to the rect.
    Fill,
}

/// A group. Membership lives on the members, as
/// [`Entity::parent`](crate::Entity::parent).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Group {
    /// The group's color.
    pub color: Option<Color>,
    /// How members are arranged.
    pub layout_mode: Option<LayoutMode>,
    /// Whether the group positions its members.
    pub managed_layout: Option<bool>,
    /// Gap between members under managed layout, in pixels.
    pub layout_gap: Option<f64>,
    /// The agent task that produced the group.
    pub source_task_id: Option<String>,
    /// Group metadata (`groupMetadata` on disk).
    pub metadata: Option<JsonMap>,
}

/// A group's arrangement of its members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// Members sit wherever they were placed.
    Freeform,
    /// Members in a row.
    Row,
    /// Members in a column.
    Column,
    /// Members in a grid.
    Grid,
}

/// A freehand drawing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Drawing {
    /// The strokes, back-to-front.
    pub strokes: Vec<Stroke>,
}

/// One stroke of a drawing. Points are in canvas space, not relative to the
/// entity rect, so moving a drawing moves its points too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stroke {
    /// Stable stroke id.
    pub id: String,
    /// Stroke color.
    pub color: Color,
    /// Stroke width in canvas units.
    pub width: f64,
    /// The path.
    pub points: Vec<Point>,
    /// Pen or highlighter. Absent means pen.
    #[serde(rename = "brushType", skip_serializing_if = "Option::is_none")]
    pub brush: Option<BrushType>,
    /// Unmodeled fields.
    #[serde(flatten)]
    pub extra: JsonMap,
}

/// The brush a stroke was drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrushType {
    /// Opaque line.
    Pen,
    /// Wide translucent line.
    Highlight,
}

/// A geometric shape with an optional inner label.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    /// Which silhouette.
    pub shape: ShapeKind,
    /// The inner label.
    pub text: String,
    /// Fill color.
    pub color: Option<Color>,
    /// Whether the fill is drawn. Absent means solid.
    pub fill_style: Option<FillStyle>,
    /// Border width in pixels.
    pub stroke_width: Option<f64>,
    /// Border style.
    pub border_style: Option<BorderStyle>,
    /// Border color.
    pub border_color: Option<Color>,
    /// Inner label size in pixels.
    pub text_size: Option<f64>,
    /// Inner label horizontal alignment. Absent means center.
    pub text_align: Option<TextAlign>,
    /// Inner label vertical alignment. Absent means middle.
    pub text_vertical_align: Option<VerticalAlign>,
    /// Named style preset.
    pub theme: Option<String>,
}

impl Shape {
    /// An unstyled shape with no label.
    pub fn new(shape: ShapeKind) -> Self {
        Self {
            shape,
            text: String::new(),
            color: None,
            fill_style: None,
            stroke_width: None,
            border_style: None,
            border_color: None,
            text_size: None,
            text_align: None,
            text_vertical_align: None,
            theme: None,
        }
    }
}

/// The shape catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    /// Square-cornered rectangle.
    Rectangle,
    /// Rounded rectangle.
    Rounded,
    /// Ellipse.
    Ellipse,
    /// Diamond.
    Diamond,
    /// Triangle.
    Triangle,
    /// Hexagon.
    Hexagon,
    /// Pill.
    Pill,
    /// Parallelogram.
    Parallelogram,
    /// Chevron.
    Chevron,
    /// Cylinder.
    Cylinder,
}

/// Whether a shape's fill is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FillStyle {
    /// Filled.
    Solid,
    /// Outline only.
    None,
}

/// A shape's border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    /// Continuous line.
    Solid,
    /// Dashed line.
    Dashed,
    /// No border.
    None,
}

/// Horizontal alignment of a shape's label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    /// Left.
    Left,
    /// Center.
    Center,
    /// Right.
    Right,
}

/// Vertical alignment of a shape's label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    /// Top.
    Top,
    /// Middle.
    Middle,
    /// Bottom.
    Bottom,
}
