//! The `Scene` display list: everything one frame draws, in one z-order.
//!
//! A [`Scene`] is a flat list of [`Item`]s painted back to front. Each item is
//! in canvas space (it moves and scales with the camera) or screen space (it
//! does not), and carries an optional clip rect and an opacity. The list
//! holds entities of all six kinds, edges, and the chrome over them:
//! selection outlines, resize handles, the marquee, comment badges and comment
//! regions.
//!
//! [`view`] builds the scene for an [`App`](specular_interact::App), with a
//! [`ViewCache`] its caller keeps between frames. The crate
//! names no renderer type, so a scene can be built and compared in a test
//! with no GPU, and one item kind can move to a different renderer later
//! (ADR 0039).

mod bounds;
mod cache;
mod color;
mod colors;
mod column;
mod fade;
mod item;
mod markdown;
mod media;
mod panel;
mod path;
mod shape;
mod text;
mod translate;
mod view;

/// The entity an item was drawn for, where whoever renders the scene has to
/// recognise it again: a page, to find the host that paints it, and a
/// Document, to report how tall its rows came out. A renderer hands it back
/// and reads nothing from it.
pub type OwnerId = specular_doc::EntityId;

pub use cache::ViewCache;
pub use color::Color;
pub use colors::{Colors, Hues, PanelColors, Shade};
pub use column::{ColumnDraw, Row, RowRule, RuleHeight};
pub use fade::PageBand;
pub use item::{Blend, Draw, Item, Scene, Space};
pub use media::{ImageDraw, ImageId, PageDraw};
pub use panel::{draw_panels, icon_svg, panel_color};
pub use path::{Dash, LineCap, LineJoin, PathCommand, PathDraw, PathStroke, PolygonDraw};
pub use shape::{EllipseDraw, RectDraw, ShadowDraw, Stroke, StrokeAlign};
pub use specular_core::{Point, Rect, Size};
pub use text::{FontFamily, SpanStyle, TextAlign, TextOverflow, TextRun, TextSpan, VerticalAlign};
pub use view::{view, view_without_chrome};
