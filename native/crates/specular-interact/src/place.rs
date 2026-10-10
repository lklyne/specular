//! The one-shot creation tools: a press that places a page, a text, a sticky
//! or a shape when it is released, and returns to the select tool.
//!
//! A click places the entity at its default size with its top-left on the
//! grid point nearest the press. A shape can also be dragged out: once the
//! drag is big enough the shape is in the document, growing with the
//! pointer, and Shift keeps it square.

use glam::DVec2;
use serde_json::json;
use specular_core::Modifiers;
use specular_doc::{
    Color, Entity, EntityId, ItemId, Kind, Page, PageSource, Rect, Shape, ShapeKind, Text,
    TextStyle, VIEWPORT_PRESETS, WidthMode, preset,
};

use crate::scroll_follow::Scrolls;
use crate::{App, Effect, Gesture, TextFrame, Tool, anchor, edit, geometry, grid, live, showing};

/// A shape drag smaller than this on either axis, in canvas units, places
/// the default size instead.
const MIN_SHAPE_DRAG: f64 = 24.0;
/// The size a clicked shape gets.
const DEFAULT_SHAPE_SIZE: DVec2 = DVec2::new(160.0, 160.0);
/// A pill in a square box is a circle, so it is placed wide.
const DEFAULT_PILL_SIZE: DVec2 = DVec2::new(200.0, 88.0);
/// The size a new text or sticky gets. The height is a sticky's height at
/// the default text size.
const DEFAULT_TEXT_SIZE: DVec2 = DVec2::new(200.0, 200.0);
/// The size a new Document gets: Electron's default for a file entity.
const DEFAULT_DOCUMENT_SIZE: DVec2 = DVec2::new(300.0, 300.0);
/// What a new page shows until it is given a URL.
const BLANK_URL: &str = "about:blank";

/// What a placement makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placing {
    /// A page.
    Page,
    /// Plain text or a sticky note.
    Text(TextStyle),
    /// A shape.
    Shape,
    /// A Document: a new markdown file and the file entity that shows it.
    Document,
}

/// A press with a one-shot creation tool, up to its release.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceDrag {
    what: Placing,
    /// The canvas point the press landed on.
    start: DVec2,
    /// The shape the drag has put in the document, once it is big enough.
    live: Option<EntityId>,
}

impl PlaceDrag {
    /// The shape being dragged out, which is in the document already.
    pub fn live(&self) -> Option<&EntityId> {
        self.live.as_ref()
    }
}

/// What `tool` places. `None` for a tool that places nothing.
const fn placing(tool: Tool) -> Option<Placing> {
    match tool {
        Tool::AddPage => Some(Placing::Page),
        Tool::AddText => Some(Placing::Text(TextStyle::Plain)),
        Tool::AddSticky => Some(Placing::Text(TextStyle::Sticky)),
        Tool::AddShape => Some(Placing::Shape),
        Tool::AddDocument => Some(Placing::Document),
        Tool::Select | Tool::Draw | Tool::Comment | Tool::Inspect => None,
    }
}

/// A press at `world` with `tool`. `None` for a tool that places nothing.
pub(crate) fn begin(tool: Tool, world: DVec2) -> Option<PlaceDrag> {
    Some(PlaceDrag {
        what: placing(tool)?,
        start: world,
        live: None,
    })
}

/// What a click with the tool in hand would make, where it would make it.
/// It is drawn faded under the pointer, so an armed tool is seen to be
/// armed and the landing spot is known before the press.
#[derive(Debug, Clone, PartialEq)]
pub enum PlacePreview {
    /// A page, a text, a sticky or a shape, as the click makes it.
    Entity(Box<Entity>),
    /// A Document, which is only its rect until its file exists.
    Document(Rect),
}

impl PlacePreview {
    /// Where the entity's text is set, for one that has text.
    pub fn text_frame(&self) -> Option<TextFrame> {
        match self {
            Self::Entity(entity) => edit::frame::of(entity, entity.rect),
            Self::Document(_) => None,
        }
    }
}

/// The id of the entity a [`PlacePreview`] holds. It is never in the
/// document.
const PREVIEW_ID: &str = "place-preview";

impl App {
    /// What a click with the tool in hand would place under the pointer, or
    /// a press that has not been released yet will. `None` with any other
    /// tool, with the pointer off the canvas, once a shape is being dragged
    /// out, and where the press would make nothing.
    pub fn place_preview(&self) -> Option<PlacePreview> {
        let tool = self.session.tool;
        let what = placing(tool)?;
        let start = match &self.session.gesture {
            None => {
                let pointer = self.session.pointer?;
                if crate::panel::builtin::over(self) {
                    return None;
                }
                let world = self.session.camera.screen_to_world(pointer).as_dvec2();
                if showing::refuses(self, tool, world) {
                    return None;
                }
                world
            }
            Some(Gesture::Place(drag)) if drag.live.is_none() => drag.start,
            Some(
                Gesture::Place(_)
                | Gesture::Move(_)
                | Gesture::Resize(_)
                | Gesture::Marquee { .. }
                | Gesture::Comment(_)
                | Gesture::TextSelect(_)
                | Gesture::EdgeDrag(_)
                | Gesture::Line(_)
                | Gesture::Draw(_),
            ) => return None,
        };
        let at = DVec2::new(grid::snap(start.x), grid::snap(start.y));
        Some(match what {
            Placing::Document => PlacePreview::Document(geometry::rect(at, DEFAULT_DOCUMENT_SIZE)),
            Placing::Page | Placing::Text(_) | Placing::Shape => {
                let mut entity = default_entity(self, what, PREVIEW_ID.into(), at);
                // A text is fitted to its text, which is empty, as soon as
                // the click starts its edit.
                if let Kind::Text(text) = &entity.kind {
                    entity.rect = edit::fitted(self, entity.rect, text);
                }
                PlacePreview::Entity(Box::new(entity))
            }
        })
    }
}

/// The rect a shape drag from `start` to `end` has reached, on the grid.
/// `square` keeps it as wide as it is tall, growing from the press. `None`
/// while the drag is too small to size a shape.
fn dragged_rect(start: DVec2, end: DVec2, square: bool) -> Option<Rect> {
    let raw = if square {
        let reach = end - start;
        let side = reach.abs().max_element();
        let origin = DVec2::new(
            if reach.x < 0.0 {
                start.x - side
            } else {
                start.x
            },
            if reach.y < 0.0 {
                start.y - side
            } else {
                start.y
            },
        );
        geometry::rect(origin, DVec2::splat(side))
    } else {
        geometry::spanning(start, end)
    };
    (raw.width >= MIN_SHAPE_DRAG && raw.height >= MIN_SHAPE_DRAG).then(|| {
        Rect::new(
            grid::snap(raw.x),
            grid::snap(raw.y),
            grid::snap(raw.width),
            grid::snap(raw.height),
        )
    })
}

/// The pointer moved, or Shift changed, with `drag` in flight.
pub(crate) fn drag(app: &mut App, drag: &mut PlaceDrag, world: DVec2, modifiers: Modifiers) {
    match drag.what {
        Placing::Page | Placing::Text(_) | Placing::Document => {}
        Placing::Shape => match dragged_rect(drag.start, world, modifiers.shift) {
            Some(rect) => {
                let id = match drag.live.take() {
                    Some(id) => id,
                    None => EntityId::from(app.fresh_id().as_str()),
                };
                let shape = shape(app, id.clone(), rect);
                live::put(&mut app.document, shape);
                drag.live = Some(id);
            }
            None => {
                if let Some(id) = drag.live.take() {
                    live::take(&mut app.document, &id);
                }
            }
        },
    }
}

/// The button came up: the entity is placed and selected, and the tool goes
/// back to select. A page or a shape is one undo step. A text or a sticky is
/// left being edited, and becomes a step when the edit ends with something
/// typed in it. A Document asks the shell for its file and is placed when
/// the file exists.
pub(crate) fn finish(app: &mut App, mut drag: PlaceDrag, effects: &mut Vec<Effect>) {
    app.session.tool = Tool::Select;
    if drag.what == Placing::Document {
        // The file comes first, and only the shell can make it. The entity
        // is placed when it answers.
        let at = DVec2::new(grid::snap(drag.start.x), grid::snap(drag.start.y));
        effects.push(Effect::CreateNote {
            rect: geometry::rect(at, DEFAULT_DOCUMENT_SIZE),
        });
        return;
    }
    let dragged = (drag.live.take()).and_then(|id| live::take(&mut app.document, &id));
    let mut entity = dragged.unwrap_or_else(|| at_default_size(app, &drag));
    let id = entity.id.clone();
    match drag.what {
        Placing::Text(_) => {
            entity.anchor = anchor::page_anchor_for(&app.document, &Scrolls::of(app), &entity);
            live::put(&mut app.document, entity);
            edit::begin(app, &id, true, effects);
        }
        Placing::Page | Placing::Shape | Placing::Document => {
            live::create(app, entity, effects);
            app.session.selection.set([ItemId::Entity(id)]);
        }
    }
}

/// The placement was abandoned: a shape being dragged out is taken back.
pub(crate) fn cancel(app: &mut App, drag: &PlaceDrag) {
    if let Some(id) = &drag.live {
        live::take(&mut app.document, id);
    }
}

/// What a click places: the default size, with its top-left on the grid point
/// nearest the press.
fn at_default_size(app: &mut App, drag: &PlaceDrag) -> Entity {
    let id = EntityId::from(app.fresh_id().as_str());
    let at = DVec2::new(grid::snap(drag.start.x), grid::snap(drag.start.y));
    default_entity(app, drag.what, id, at)
}

/// What `what` is at its default size with its top-left at `at`.
fn default_entity(app: &App, what: Placing, id: EntityId, at: DVec2) -> Entity {
    match what {
        // A Document is placed by the shell's answer, not from here.
        Placing::Page | Placing::Document => page(app, id, at),
        Placing::Text(style) => text(app, id, at, style),
        Placing::Shape => {
            let size = match app.tool_defaults.shape.kind {
                ShapeKind::Pill => DEFAULT_PILL_SIZE,
                ShapeKind::Rectangle
                | ShapeKind::Rounded
                | ShapeKind::Ellipse
                | ShapeKind::Diamond
                | ShapeKind::Triangle
                | ShapeKind::Hexagon
                | ShapeKind::Parallelogram
                | ShapeKind::Chevron
                | ShapeKind::Cylinder => DEFAULT_SHAPE_SIZE,
            };
            shape(app, id, geometry::rect(at, size))
        }
    }
}

fn shape(app: &App, id: EntityId, rect: Rect) -> Entity {
    let defaults = &app.tool_defaults.shape;
    let shape = Shape {
        color: Some(defaults.color.clone()),
        stroke_width: Some(defaults.stroke_width),
        text_size: Some(defaults.text_size),
        ..Shape::new(defaults.kind)
    };
    Entity::new(id, rect, Kind::Shape(shape))
}

pub(crate) fn text(app: &App, id: EntityId, at: DVec2, style: TextStyle) -> Entity {
    let defaults = &app.tool_defaults;
    // Plain text's color is its ink, which follows the theme unless one was
    // picked. A sticky's is its card.
    let (color, size, font, width_mode) = match style {
        TextStyle::Plain => (
            defaults.text.color.clone().unwrap_or(Color::Neutral),
            defaults.text.size,
            defaults.text.font,
            WidthMode::Auto,
        ),
        TextStyle::Sticky => (
            defaults.sticky.color.clone(),
            defaults.sticky.size,
            defaults.sticky.font,
            WidthMode::Fixed,
        ),
    };
    let text = Text {
        text: String::new(),
        color: Some(color),
        style: Some(style),
        width_mode: Some(width_mode),
        size: Some(size),
        font: Some(font),
    };
    Entity::new(id, geometry::rect(at, DEFAULT_TEXT_SIZE), Kind::Text(text))
}

/// A page at the page tool's preset with its device in a free spot of the
/// canvas: beside the selection when there is one, as a page made through
/// the API is placed.
pub(crate) fn page_in_free_spot(app: &mut App) -> Entity {
    let id = EntityId::from(app.fresh_id().as_str());
    let sized = page(app, id.clone(), DVec2::ZERO);
    // At the origin the screen's corner is the device's insets.
    let device = geometry::size(sized.rect) + geometry::origin(sized.rect) * 2.0;
    let spot = crate::free_spot::place(app, device.x, device.y, true);
    page(app, id, DVec2::new(spot.x, spot.y))
}

/// A page at the preset the page tool is set to (the first, an iPhone SE,
/// until another is picked), turned across for a preset wider than it is
/// tall, as Electron's `defaultOrientationForDevice` has it.
pub(crate) fn page(app: &App, id: EntityId, at: DVec2) -> Entity {
    let defaults = app.tool_defaults.page;
    let (index, chosen) = match preset(u64::from(defaults.preset)) {
        Some(chosen) => (defaults.preset, chosen),
        None => (0, &VIEWPORT_PRESETS[0]),
    };
    let orientation = if chosen.width > chosen.height {
        "landscape"
    } else {
        "portrait"
    };
    let metadata = json!({
        "createdFrom": "add_from_toolbar",
        "deviceOrientation": orientation,
        "showDeviceFrame": true,
        "deviceId": chosen.device_id,
    });
    let mut page = Page {
        url: BLANK_URL.to_owned(),
        preset_index: Some(index),
        source: Some(PageSource::Manual),
        metadata: metadata.as_object().cloned(),
        ..Page::default()
    };
    let mut rect = geometry::rect(at, DVec2::new(chosen.width, chosen.height));
    if defaults.custom {
        crate::property::make_custom(&mut page, &mut rect);
    }
    // `at` is where the device's corner goes, and the screen sits in from it.
    if let Some(shell) = page.shell() {
        rect.x += shell.insets.left;
        rect.y += shell.insets.top;
    }
    Entity::new(id, rect, Kind::Page(page))
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: DVec2 = DVec2::new(105.0, 95.0);

    #[test]
    fn a_drag_spans_the_press_and_the_pointer_on_the_grid() {
        // (pointer, shift, rect). A rect under one grid cell on a side is
        // none. Shift makes a square of the longer reach that grows from the
        // press, which stays the opposite corner going up and left.
        let rows = [
            (
                (251.0, 178.0),
                false,
                Some(Rect::new(100.0, 100.0, 140.0, 80.0)),
            ),
            (
                (-10.0, 30.0),
                false,
                Some(Rect::new(0.0, 40.0, 120.0, 60.0)),
            ),
            ((300.0, 110.0), false, None),
            ((120.0, 300.0), false, None),
            ((105.0, 95.0), true, None),
            (
                (205.0, 125.0),
                true,
                Some(Rect::new(100.0, 100.0, 100.0, 100.0)),
            ),
            ((85.0, -5.0), true, Some(Rect::new(0.0, 0.0, 100.0, 100.0))),
        ];
        for ((x, y), shift, want) in rows {
            assert_eq!(
                dragged_rect(START, DVec2::new(x, y), shift),
                want,
                "({x}, {y}) shift {shift}"
            );
        }
    }
}
