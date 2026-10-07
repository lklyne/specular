//! [`TextFrame`]: where an entity's text sits inside its rect and how it is
//! set, and the size a text entity takes to fit its text.
//!
//! The numbers are the Electron editors': a sticky pads its text by 8 all
//! round, plain text keeps 8 clear on the right for the caret, and a shape
//! label is padded 12 by 8 inside the shape's label box.

use glam::DVec2;
use specular_doc::{
    Entity, Kind, Rect, Shape, ShapeKind, Text, TextAlign, TextFont, TextStyle, VerticalAlign,
    WidthMode,
};

use super::layout::LineBox;
use super::measure::{TextLayout, TextMeasure, TextSpec};

/// Text size when the entity sets none.
const DEFAULT_SIZE: f32 = 14.0;
/// Room kept clear on the right of plain text, so a caret fits.
const PLAIN_RIGHT_PADDING: f32 = 8.0;
/// The least an auto-width plain text is wide and any plain text is tall.
const PLAIN_MIN_WIDTH: f32 = 64.0;
const PLAIN_MIN_HEIGHT: f32 = 18.0;
/// What an empty plain text shows, which it stays wide enough for.
const PLAIN_PLACEHOLDER: &str = "Add text";
/// Space between a sticky note's edge and its text.
const STICKY_PADDING: f32 = 8.0;
/// A sticky is never shorter than this at the default text size, and the
/// floor scales with the size.
const STICKY_BASE_HEIGHT: f32 = 200.0;
const LABEL_LINE_HEIGHT: f32 = 1.4;
const LABEL_PADDING: DVec2 = DVec2::new(12.0, 8.0);

/// Where an entity's text is laid out, in canvas space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextFrame {
    /// Top-left of the layout box, or the anchor on an axis with no extent.
    pub origin: DVec2,
    /// How the text is set.
    pub spec: TextSpec,
    /// Height of the layout box, for vertical alignment. `None` starts the
    /// text at the origin.
    pub box_height: Option<f32>,
    /// Where the block of lines sits in the box's height.
    pub vertical: VerticalAlign,
}

/// Line height as a multiple of the size: roomy for body text, tightening
/// as headings grow.
fn text_line_height(size: f32) -> f32 {
    (1.5 - (size - 14.0) / 82.0 * 0.4).clamp(1.1, 1.5)
}

/// The box a shape's label is set in: the rect, pulled in where the
/// silhouette is narrower than it.
fn label_box(kind: ShapeKind, rect: Rect) -> Rect {
    let (x, y, width, height) = match kind {
        ShapeKind::Diamond => (25.0, 25.0, 50.0, 50.0),
        ShapeKind::Triangle => (20.0, 48.0, 60.0, 42.0),
        ShapeKind::Chevron => (5.0, 15.0, 60.0, 70.0),
        ShapeKind::Cylinder => (8.0, 28.0, 84.0, 58.0),
        ShapeKind::Rectangle
        | ShapeKind::Rounded
        | ShapeKind::Ellipse
        | ShapeKind::Hexagon
        | ShapeKind::Pill
        | ShapeKind::Parallelogram => return rect,
    };
    Rect::new(
        rect.x + rect.width * x / 100.0,
        rect.y + rect.height * y / 100.0,
        rect.width * width / 100.0,
        rect.height * height / 100.0,
    )
}

fn text_frame(rect: Rect, text: &Text) -> TextFrame {
    let size = text.size.map_or(DEFAULT_SIZE, |size| size as f32);
    let (inset, wrap_width) = match (text.resolved_style(), text.resolved_width_mode()) {
        (TextStyle::Plain, WidthMode::Auto) => (0.0, None),
        (TextStyle::Plain, WidthMode::Fixed) => {
            (0.0, Some(rect.width as f32 - PLAIN_RIGHT_PADDING))
        }
        (TextStyle::Sticky, _) => (
            STICKY_PADDING,
            Some(rect.width as f32 - STICKY_PADDING * 2.0),
        ),
    };
    TextFrame {
        origin: DVec2::new(rect.x, rect.y) + DVec2::splat(f64::from(inset)),
        spec: TextSpec {
            font: text.font.unwrap_or(TextFont::Sans),
            size,
            line_height: size * text_line_height(size),
            wrap_width: wrap_width.map(|width| width.max(0.0)),
            align: TextAlign::Left,
        },
        box_height: None,
        vertical: VerticalAlign::Top,
    }
}

fn label_frame(rect: Rect, shape: &Shape) -> TextFrame {
    let within = label_box(shape.shape, rect);
    let size = shape.text_size.map_or(DEFAULT_SIZE, |size| size as f32);
    let inner = DVec2::new(within.width, within.height) - LABEL_PADDING * 2.0;
    TextFrame {
        origin: DVec2::new(within.x, within.y) + LABEL_PADDING,
        spec: TextSpec {
            font: TextFont::Sans,
            size,
            line_height: size * LABEL_LINE_HEIGHT,
            wrap_width: Some(inner.x.max(0.0) as f32),
            align: shape.text_align.unwrap_or(TextAlign::Center),
        },
        box_height: Some(inner.y.max(0.0) as f32),
        vertical: shape.text_vertical_align.unwrap_or(VerticalAlign::Middle),
    }
}

/// The frame of `entity`'s text, or `None` for a kind with no text to edit.
pub(crate) fn of(entity: &Entity) -> Option<TextFrame> {
    match &entity.kind {
        Kind::Text(text) => Some(text_frame(entity.rect, text)),
        Kind::Shape(shape) => Some(label_frame(entity.rect, shape)),
        Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => None,
    }
}

impl TextFrame {
    /// How far below the origin the first line starts.
    fn lead(&self, layout: &TextLayout) -> f32 {
        let spare = self
            .box_height
            .map_or(0.0, |height| height - layout.height());
        match self.vertical {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Middle => spare / 2.0,
            VerticalAlign::Bottom => spare,
        }
    }

    /// The offset nearest the canvas point `world`.
    pub(crate) fn offset_at(&self, layout: &TextLayout, world: DVec2) -> usize {
        let local = (world - self.origin).as_vec2();
        layout.offset_at_point(local.x, local.y - self.lead(layout))
    }

    /// A box of `layout` as a canvas rect.
    pub(crate) fn rect_of(&self, layout: &TextLayout, line: LineBox) -> Rect {
        Rect::new(
            self.origin.x + f64::from(line.left),
            self.origin.y + f64::from(line.top + self.lead(layout)),
            f64::from(line.right - line.left),
            f64::from(line.height),
        )
    }
}

/// The rect a text entity takes to fit `working`, its text as edited so
/// far: a sticky and a fixed-width text grow downward, and an auto-width
/// text hugs its lines both ways. Sizes are whole canvas units.
pub(crate) fn fitted(
    entity: &Entity,
    text: &Text,
    working: &str,
    measure: &dyn TextMeasure,
) -> Rect {
    let frame = text_frame(entity.rect, text);
    let layout = measure.layout(working, &frame.spec);
    let rect = entity.rect;
    let (width, height) = match (text.resolved_style(), text.resolved_width_mode()) {
        (TextStyle::Plain, WidthMode::Auto) => {
            let floor = if working.is_empty() {
                let prompt = measure.layout(PLAIN_PLACEHOLDER, &frame.spec).width();
                PLAIN_MIN_WIDTH.max(prompt.ceil() + PLAIN_RIGHT_PADDING)
            } else {
                PLAIN_MIN_WIDTH
            };
            (
                f64::from(floor.max((layout.width() + PLAIN_RIGHT_PADDING).round())),
                f64::from(PLAIN_MIN_HEIGHT.max(layout.height().round())),
            )
        }
        (TextStyle::Plain, WidthMode::Fixed) => (
            rect.width,
            f64::from(PLAIN_MIN_HEIGHT.max(layout.height()).ceil()),
        ),
        (TextStyle::Sticky, _) => {
            let floor = STICKY_BASE_HEIGHT * frame.spec.size / DEFAULT_SIZE;
            let content = layout.height() + STICKY_PADDING * 2.0;
            (rect.width, f64::from(floor.max(content).ceil()))
        }
    };
    Rect::new(rect.x, rect.y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_height_tightens_as_text_grows() {
        let heights = [14.0, 32.0, 96.0].map(|size| (text_line_height(size) * 1000.0).round());
        assert_eq!(heights, [1500.0, 1412.0, 1100.0]);
    }

    #[test]
    fn a_label_box_is_inset_where_the_silhouette_is_narrow() {
        let rect = Rect::new(100.0, 200.0, 200.0, 100.0);
        assert_eq!(
            label_box(ShapeKind::Diamond, rect),
            Rect::new(150.0, 225.0, 100.0, 50.0)
        );
        assert_eq!(label_box(ShapeKind::Hexagon, rect), rect);
    }
}
