//! [`Scene`], [`Item`] and the [`Draw`] variants.

use crate::{
    ColumnDraw, EllipseDraw, ImageDraw, PageDraw, PathDraw, PolygonDraw, Rect, RectDraw,
    ShadowDraw, TextRun,
};

/// The coordinate space an item's geometry is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Space {
    /// Canvas units. The item moves with the pan and scales with the zoom,
    /// stroke widths and font sizes included.
    Canvas,
    /// Logical screen pixels from the viewport's top-left. The camera does
    /// not affect the item. Chrome that hugs an entity but keeps its pixel
    /// size (outlines, handles, badges) is projected by `view` and drawn here.
    Screen,
}

/// How an item's colour meets what is already painted under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Blend {
    /// Painted over, by its alpha.
    #[default]
    Normal,
    /// Multiplied in, as ink on paper: it darkens what is lighter than
    /// itself and leaves what is darker, so black text under a highlighter
    /// stays black. Paths and polygons honour it. Every other draw paints
    /// [`Normal`](Self::Normal).
    Multiply,
}

/// What an item draws.
#[derive(Debug, Clone, PartialEq)]
pub enum Draw {
    /// A live page's frame.
    Page(PageDraw),
    /// The blurred shadow of a rounded rect.
    Shadow(ShadowDraw),
    /// A rounded rect.
    Rect(RectDraw),
    /// An ellipse.
    Ellipse(EllipseDraw),
    /// A closed polygon.
    Polygon(PolygonDraw),
    /// A stroked and/or filled path.
    Path(PathDraw),
    /// A block of text.
    Text(TextRun),
    /// Rows of text stacked top to bottom.
    Column(ColumnDraw),
    /// An image.
    Image(ImageDraw),
}

macro_rules! draw_from {
    ($($variant:ident($payload:ty)),* $(,)?) => {
        $(impl From<$payload> for Draw {
            fn from(draw: $payload) -> Self {
                Self::$variant(draw)
            }
        })*
    };
}

draw_from!(
    Page(PageDraw),
    Shadow(ShadowDraw),
    Rect(RectDraw),
    Ellipse(EllipseDraw),
    Polygon(PolygonDraw),
    Path(PathDraw),
    Text(TextRun),
    Column(ColumnDraw),
    Image(ImageDraw),
);

/// One entry of the display list.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The space `draw` and `clip` are in.
    pub space: Space,
    /// Nothing is drawn outside this rect, when set.
    pub clip: Option<Rect>,
    /// Multiplies the item's alpha; 1 leaves it as drawn.
    pub opacity: f32,
    /// How it is laid over what is under it.
    pub blend: Blend,
    /// What to draw.
    pub draw: Draw,
}

impl Item {
    /// An unclipped, fully opaque item in canvas space.
    pub fn canvas(draw: impl Into<Draw>) -> Self {
        Self::new(Space::Canvas, draw)
    }

    /// An unclipped, fully opaque item in screen space.
    pub fn screen(draw: impl Into<Draw>) -> Self {
        Self::new(Space::Screen, draw)
    }

    fn new(space: Space, draw: impl Into<Draw>) -> Self {
        Self {
            space,
            clip: None,
            opacity: 1.0,
            blend: Blend::Normal,
            draw: draw.into(),
        }
    }

    /// The same item clipped to `clip`, a rect in the item's space.
    #[must_use]
    pub fn clipped(self, clip: Rect) -> Self {
        Self {
            clip: Some(clip),
            ..self
        }
    }

    /// The same item laid over what is under it by `blend`.
    #[must_use]
    pub fn with_blend(self, blend: Blend) -> Self {
        Self { blend, ..self }
    }

    /// The same item at `opacity`.
    #[must_use]
    pub fn with_opacity(self, opacity: f32) -> Self {
        Self { opacity, ..self }
    }
}

/// Everything one frame draws, back to front.
///
/// Pages and every other item share the one order: an item after a page in
/// the list is painted over it, an item before it is painted under.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    /// The items in paint order.
    pub items: Vec<Item>,
}

impl Scene {
    /// An empty scene.
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Adds an item on top of everything pushed so far.
    pub fn push(&mut self, item: Item) {
        self.items.push(item);
    }
}

impl FromIterator<Item> for Scene {
    fn from_iter<I: IntoIterator<Item = Item>>(items: I) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }
}

impl Extend<Item> for Scene {
    fn extend<I: IntoIterator<Item = Item>>(&mut self, items: I) {
        self.items.extend(items);
    }
}
