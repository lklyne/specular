//! The glyphs of the built-in panels, as scene paths.
//!
//! Each [`Icon`] is a [`Glyph`]: a view box and layers of SVG path data,
//! rects and discs, each filled and stroked in a [`Paint`]. The data is the
//! Electron app's own: Lucide's node lists, the toolbar's SVG assets and
//! the inline glyphs of `CustomIcons.tsx`. A gradient is drawn as its
//! middle color, and the filters (drop shadows, inner shadows) and the fade
//! at the foot of the pens are left out.

mod lucide;
mod paths_pens;
mod paths_popup;
mod paths_tools;
mod popup;
mod svg;
mod tools;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use specular_interact::Icon;

use crate::{Color, Item, PathCommand, PathDraw, PathStroke, Point, Rect};

/// The dark line of the toolbar glyphs: `#45403C`.
const OUTLINE: Paint = Paint::Hex(0x45_403c);
/// The pale body of the toolbar glyphs: the middle of their `#F4F4F4` to
/// `#DCDCDC` gradient.
const BODY: Paint = Paint::Hex(0xe8_e8e8);
/// The ink of a pen whose strokes do not share one: `DEFAULT_PEN_INK`.
const NO_INK: Color = Color::rgb(0xbd, 0x4b, 0xe5);
/// How far toward white the paper of the sticky and shape glyphs is taken
/// from the tool's color: the middle of `lightenHex(tint, 0.45)` to
/// `lightenHex(tint, 0.15)`.
const PAPER_LIGHTEN: f32 = 0.3;

/// What a layer is filled or stroked in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Paint {
    /// Nothing.
    None,
    /// The text color of the control the glyph is on: `currentColor`.
    Current,
    /// A fixed color, `0xRRGGBB`.
    Hex(u32),
    /// A fixed color at an alpha.
    HexA(u32, u8),
    /// The color the glyph shows: a pen's ink.
    Tint,
    /// That color taken toward white, as the paper of a sticky.
    Paper,
    /// The line of a popup pen: `#797875`, or `#18181B` when it is the
    /// brush in use.
    PenLine,
}

/// The outline of a layer.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    /// SVG path data.
    Path(&'static str),
    /// A rect at `x, y` of `width, height` with corners of `radius`.
    Rect(f32, f32, f32, f32, f32),
    /// A disc at `cx, cy` of `radius`.
    Disc(f32, f32, f32),
}

/// A transform applied to a layer's outline, in the glyph's own units.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Turn {
    /// As drawn.
    None,
    /// The SVG `matrix(a b c d e f)`.
    Matrix([f32; 6]),
    /// The SVG `rotate(degrees cx cy)`.
    Rotate(f32, f32, f32),
}

/// One painted outline of a glyph.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Layer {
    shape: Shape,
    fill: Paint,
    stroke: Paint,
    /// The stroke's width in the glyph's own units.
    width: f32,
    turn: Turn,
}

impl Layer {
    const fn new(shape: Shape, fill: Paint, stroke: Paint, width: f32) -> Self {
        Self {
            shape,
            fill,
            stroke,
            width,
            turn: Turn::None,
        }
    }

    const fn fill(shape: Shape, fill: Paint) -> Self {
        Self::new(shape, fill, Paint::None, 0.0)
    }

    const fn stroke(shape: Shape, stroke: Paint, width: f32) -> Self {
        Self::new(shape, Paint::None, stroke, width)
    }

    const fn turned(self, turn: Turn) -> Self {
        Self { turn, ..self }
    }
}

/// A glyph: layers, back to front, in a view box.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Glyph {
    /// The SVG `viewBox`: `x, y, width, height`.
    view: [f32; 4],
    /// Whether the glyph is cut off at its view box: a pen runs out of the
    /// bottom of its frame.
    clipped: bool,
    layers: &'static [Layer],
}

impl Glyph {
    const fn square(side: f32, layers: &'static [Layer]) -> Self {
        Self {
            view: [0.0, 0.0, side, side],
            clipped: false,
            layers,
        }
    }
}

/// The colors a glyph is drawn in on one control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Inks {
    /// The control's text color.
    pub(super) current: Color,
    /// The color the glyph shows, when it shows one.
    pub(super) tint: Option<Color>,
    /// Whether the control is on.
    pub(super) on: bool,
}

impl Inks {
    /// One color, for a glyph that shows none of its own.
    pub(super) const fn plain(current: Color) -> Self {
        Self {
            current,
            tint: None,
            on: false,
        }
    }
}

fn glyph(icon: Icon) -> Glyph {
    match icon {
        Icon::SelectTool => tools::SELECT_TOOL,
        Icon::PageTool => tools::PAGE_TOOL,
        Icon::TextTool => tools::TEXT_GLYPH,
        Icon::StickyTool => tools::STICKY_TOOL,
        Icon::DocumentTool => tools::DOCUMENT_TOOL,
        Icon::ShapeTool => tools::SHAPE_TOOL,
        Icon::DrawPenTool => tools::PEN_TOOL,
        Icon::DrawHighlightTool => tools::MARKER_TOOL,
        Icon::CommentTool => tools::COMMENT_TOOL,
        Icon::Shape(kind) => popup::shape(kind),
        Icon::AlignLeft => lucide::ALIGN_LEFT,
        Icon::AlignCenter => lucide::ALIGN_CENTER,
        Icon::AlignRight => lucide::ALIGN_RIGHT,
        Icon::BrushPen => popup::BRUSH_PEN,
        Icon::BrushHighlighter => popup::BRUSH_HIGHLIGHTER,
        Icon::StrokeThin => popup::STROKE_THIN_SAMPLE,
        Icon::StrokeThick => popup::STROKE_THICK_SAMPLE,
        Icon::Border => popup::BORDER,
        Icon::LineSolid => popup::LINE_SOLID,
        Icon::LineDashed => popup::LINE_DASHED,
        Icon::Ban => lucide::BAN,
        Icon::ArrowStart => lucide::ARROW_LEFT,
        Icon::ArrowEnd => lucide::ARROW_RIGHT,
        Icon::Trash => lucide::TRASH,
        Icon::Bold => lucide::BOLD,
        Icon::Strikethrough => lucide::STRIKETHROUGH,
        Icon::BulletList => lucide::LIST,
        Icon::Device => lucide::SMARTPHONE,
        Icon::Rotate => popup::ROTATE,
        Icon::SchemeSystem => popup::SCHEME_SYSTEM,
        Icon::SchemeLight => popup::SCHEME_LIGHT,
        Icon::SchemeDark => popup::SCHEME_DARK,
    }
}

fn hex(value: u32, alpha: u8) -> Color {
    let [_, r, g, b] = value.to_be_bytes();
    Color::rgba(r, g, b, alpha)
}

fn paint(paint: Paint, inks: Inks) -> Option<Color> {
    let tint = inks.tint.unwrap_or(NO_INK);
    match paint {
        Paint::None => None,
        Paint::Current => Some(inks.current),
        Paint::Hex(value) => Some(hex(value, 255)),
        Paint::HexA(value, alpha) => Some(hex(value, alpha)),
        Paint::Tint => Some(tint),
        Paint::Paper => Some(crate::view::palette::lighten(tint, PAPER_LIGHTEN)),
        Paint::PenLine if inks.on => Some(hex(0x18_181b, 255)),
        Paint::PenLine => Some(hex(0x79_7875, 255)),
    }
}

/// The commands of path data, parsed once: the same few dozen strings are
/// drawn every frame.
fn parsed(d: &'static str) -> Arc<[PathCommand]> {
    static PARSED: LazyLock<Mutex<HashMap<usize, Arc<[PathCommand]>>>> =
        LazyLock::new(Mutex::default);
    let mut parsed = PARSED.lock().unwrap_or_else(PoisonError::into_inner);
    // Each string is one constant, so where it lives names it.
    let commands = parsed
        .entry(d.as_ptr() as usize)
        .or_insert_with(|| svg::parse(d).into());
    Arc::clone(commands)
}

/// A rect with rounded corners as a path, drawn clockwise from the top.
fn rounded(left: f32, top: f32, width: f32, height: f32, radius: f32) -> Arc<[PathCommand]> {
    let radius = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    let (across, down) = (width - 2.0 * radius, height - 2.0 * radius);
    let corner = format!("a{radius} {radius} 0 0 1");
    let data = format!(
        "M{} {top}h{across}{corner} {radius} {radius}v{down}{corner} -{radius} {radius}\
         h-{across}{corner} -{radius} -{radius}v-{down}{corner} {radius} -{radius}z",
        left + radius
    );
    svg::parse(&data).into()
}

fn outline(shape: Shape) -> Arc<[PathCommand]> {
    match shape {
        Shape::Path(d) => parsed(d),
        Shape::Rect(x, y, width, height, radius) => rounded(x, y, width, height, radius),
        Shape::Disc(cx, cy, radius) => {
            rounded(cx - radius, cy - radius, radius * 2.0, radius * 2.0, radius)
        }
    }
}

/// The `matrix(a b c d e f)` of `turn`.
fn matrix(turn: Turn) -> [f32; 6] {
    match turn {
        Turn::None => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        Turn::Matrix(matrix) => matrix,
        Turn::Rotate(degrees, cx, cy) => {
            let (sin, cos) = degrees.to_radians().sin_cos();
            [
                cos,
                sin,
                -sin,
                cos,
                cx - cx * cos + cy * sin,
                cy - cx * sin - cy * cos,
            ]
        }
    }
}

/// `icon` fitted into `rect` keeping its proportions, as scene items.
pub(super) fn draw(icon: Icon, rect: Rect, inks: Inks, out: &mut Vec<Item>) {
    fit(glyph(icon), rect, inks, out);
}

/// The down chevron of a dropdown, fitted into `rect`.
pub(super) fn chevron(rect: Rect, color: Color, out: &mut Vec<Item>) {
    fit(lucide::CHEVRON, rect, Inks::plain(color), out);
}

/// The check beside a selected row, fitted into `rect`.
pub(super) fn check(rect: Rect, color: Color, out: &mut Vec<Item>) {
    fit(lucide::CHECK, rect, Inks::plain(color), out);
}

fn fit(glyph: Glyph, rect: Rect, inks: Inks, out: &mut Vec<Item>) {
    let [view_x, view_y, view_width, view_height] = glyph.view;
    let scale = (rect.width / view_width).min(rect.height / view_height);
    let left = rect.x + (rect.width - view_width * scale) / 2.0 - view_x * scale;
    let top = rect.y + (rect.height - view_height * scale) / 2.0 - view_y * scale;
    for layer in glyph.layers {
        let [xx, yx, xy, yy, dx, dy] = matrix(layer.turn);
        let place = |point: Point| {
            let across = xx * point.x + xy * point.y + dx;
            let down = yx * point.x + yy * point.y + dy;
            Point::new(left + across * scale, top + down * scale)
        };
        let commands = (outline(layer.shape).iter())
            .map(|command| match *command {
                PathCommand::MoveTo(to) => PathCommand::MoveTo(place(to)),
                PathCommand::LineTo(to) => PathCommand::LineTo(place(to)),
                PathCommand::QuadTo { control, to } => PathCommand::QuadTo {
                    control: place(control),
                    to: place(to),
                },
                PathCommand::CubicTo {
                    control1,
                    control2,
                    to,
                } => PathCommand::CubicTo {
                    control1: place(control1),
                    control2: place(control2),
                    to: place(to),
                },
                PathCommand::Close => PathCommand::Close,
            })
            .collect();
        let stroke =
            paint(layer.stroke, inks).map(|color| PathStroke::new(color, layer.width * scale));
        let item = Item::screen(PathDraw {
            commands,
            fill: paint(layer.fill, inks),
            stroke,
        });
        out.push(if glyph.clipped {
            item.clipped(rect)
        } else {
            item
        });
    }
}

#[cfg(test)]
mod tests;
