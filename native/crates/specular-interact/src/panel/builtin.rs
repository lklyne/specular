//! The chrome laid out, hit-tested and clicked with no UI library: the
//! stand-in for a panel renderer.
//!
//! The chrome is three rows across the top of the window (the tab row, the
//! tool row and the dock) with the sidebar under them at the left.
//! [`layout`] turns the models into rects in logical screen pixels, each
//! with what a drawer paints there, so drawing needs no arithmetic of its
//! own. [`PanelUi`] is the state a renderer keeps against
//! [`ControlId`](super::ControlId): which dropdown is open and what the
//! pointer is over. A press on a panel never reaches the canvas.
//!
//! Everything is off until [`Event::BuiltinPanels`](crate::Event) turns it
//! on, so a shell that draws the models itself is not hit-tested against
//! panels it does not show.

mod cache;
mod context;
mod controls;
mod dock;
mod dropdown;
mod field;
mod metrics;
mod node;
mod place;
mod route;
mod rows;
mod scroll;
mod sidebar;
mod strip;
mod toolbar;
mod trigger;

use std::sync::Arc;

use glam::Vec2;

pub use self::cache::LayoutCache;
pub(crate) use self::cache::{
    forget as forget_layout, forget_unless as forget_layout_unless, keeps_layout,
};
pub use self::context::ContextMenu;
pub(crate) use self::context::{open as open_menu, open_for_press as open_menu_for_press};
pub(crate) use self::field::{field_box, field_text_area};
pub use self::metrics::{
    CHROME_HEIGHT, DOCK_ROW, FIELD_HEIGHT, FIELD_LINE, FIELD_TEXT, SHELL_CHROME_HEIGHT, TAB_ROW,
    TOOL_ROW,
};
pub use self::node::{
    Chrome, Input, InputFocus, Node, NodeState, Panel, PanelRect, Part, Pointing, Surface, Tint,
    Tone,
};
pub(crate) use self::route::{cancel, hit, on_pointer, over, over_field, tidy};
pub(crate) use self::scroll::on_wheel;
pub(crate) use self::sidebar::picked;
use super::{Control, ControlId, ControlsModel, Dropdown, ToolbarModel, ToolbarSection};
use crate::App;

/// What the built-in panels remember between events.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelUi {
    /// Whether the built-in panels are shown, hit and clicked at all.
    pub built_in: bool,
    /// Whether only the context menu is built in: the chrome and the
    /// sidebar are someone else's to draw.
    pub menu_only: bool,
    /// The dropdown that is open.
    pub open: Option<ControlId>,
    /// The control under the pointer.
    pub hover: Option<ControlId>,
    /// The control a press landed on and has not been released from.
    pub pressed: Option<ControlId>,
    /// How far the sidebar's list is scrolled, in pixels. It is kept inside
    /// what the list's content allows whenever the list is laid out.
    pub sidebar_scroll: f32,
    /// The context menu that is open, if one is.
    pub menu: Option<ContextMenu>,
    /// The item last picked in the sidebar, which a shift-click selects a
    /// run from.
    pub anchor: Option<specular_doc::ItemId>,
    /// The layout kept between reads.
    pub(crate) cache: LayoutCache,
}

/// Turns the built-in panels on or off, forgetting everything they held.
pub(crate) fn turn(app: &mut App, built_in: bool) {
    app.session.panel = PanelUi {
        built_in,
        ..PanelUi::default()
    };
}

impl PanelUi {
    /// How much of the viewport's top edge the chrome covers: the built-in
    /// rows, or the rows of the shell that draws its own.
    pub const fn chrome_height(&self) -> f32 {
        match (self.built_in, self.menu_only) {
            (false, _) => 0.0,
            (true, true) => SHELL_CHROME_HEIGHT,
            (true, false) => CHROME_HEIGHT,
        }
    }

    /// Only the context menu, with nothing open or hovered.
    pub fn menu_alone() -> Self {
        Self {
            built_in: true,
            menu_only: true,
            ..Self::default()
        }
    }
}

/// The panels as laid out for one frame. Empty when the built-in panels are
/// off.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelLayout {
    /// The sidebar's frame, while it is shown.
    pub sidebar: Option<Panel>,
    /// The sidebar's scrolling list, seen through its box.
    pub sidebar_list: Option<Panel>,
    /// The tab row, the strip across the top of the viewport.
    pub tabs: Option<Panel>,
    /// The toolbar, the row under the tabs.
    pub toolbar: Option<Panel>,
    /// The dock, the row under the toolbar: the controls of the tool in
    /// hand or of the selection, or an empty bar.
    pub dock: Option<Panel>,
    /// The list under the open dropdown, or the open context menu.
    pub dropdown: Option<Panel>,
}

/// What a point on a panel lands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelHit {
    /// The panel.
    pub surface: Surface,
    /// The control there, if the point is on one.
    pub control: Option<ControlId>,
}

impl PanelLayout {
    /// The panels from the back to the front.
    pub fn panels(&self) -> impl Iterator<Item = &Panel> {
        [
            &self.sidebar,
            &self.sidebar_list,
            &self.tabs,
            &self.toolbar,
            &self.dock,
            &self.dropdown,
        ]
        .into_iter()
        .flatten()
    }

    /// The names of the controls shown, from the back panel to the front.
    pub fn controls(&self) -> impl Iterator<Item = &ControlId> {
        self.panels()
            .flat_map(|panel| &panel.nodes)
            .filter_map(|node| node.id.as_ref())
    }

    /// The node of the control named `id`.
    pub fn node(&self, id: &ControlId) -> Option<&Node> {
        self.panels()
            .flat_map(|panel| &panel.nodes)
            .find(|node| node.id.as_ref() == Some(id))
    }

    /// What is under `point`: the frontmost panel there, and its control.
    pub fn hit(&self, point: Vec2) -> Option<PanelHit> {
        let panels: Vec<&Panel> = self.panels().collect();
        let panel = panels
            .into_iter()
            .rev()
            .find(|panel| panel.rect.contains(point))?;
        let control = (panel.nodes.iter().rev())
            .find(|node| node.id.is_some() && node.rect.contains(point))
            .and_then(|node| node.id.clone());
        Some(PanelHit {
            surface: panel.surface,
            control,
        })
    }
}

/// What laying out needs to know: the app for its text measure, and the
/// pointer state to mark on each control.
pub(super) struct Ctx<'a> {
    app: &'a App,
    ui: &'a PanelUi,
}

impl Ctx<'_> {
    /// How much of the viewport's left edge the sidebar covers.
    fn left(&self) -> f32 {
        self.app.covered_left()
    }

    /// How a control is drawn now. A control that cannot be used takes no
    /// hover or press.
    fn state(&self, id: &ControlId, enabled: bool, on: bool) -> NodeState {
        let over = self.ui.hover.as_ref() == Some(id);
        let held = self.ui.pressed.as_ref() == Some(id);
        NodeState {
            enabled,
            on,
            pointing: Pointing::of(enabled, over, held),
            dimmed: false,
        }
    }

    /// The width of one line of panel text, rounded up to a whole pixel so a
    /// label never sits on a fraction.
    fn text_width(&self, text: &str, font: specular_doc::TextFont) -> f32 {
        let spec = crate::TextSpec {
            font: crate::edit::frame::font(font),
            size: metrics::TEXT_SIZE,
            line_height: metrics::TEXT_LINE,
            wrap_width: None,
            align: specular_core::text::TextAlign::Left,
        };
        let layout = self.app.text_measure().layout(text, &spec);
        let width = layout.lines.first().map_or(0.0, |line| {
            let start = line.stops.first().map_or(0.0, |stop| stop.x);
            line.stops.last().map_or(0.0, |stop| stop.x) - start
        });
        width.ceil()
    }
}

/// Whether `controls` holds the dropdown named `id` at its top level.
fn holds<'a>(controls: &'a [Control], id: &ControlId) -> Option<&'a Dropdown> {
    controls.iter().find_map(|control| match control {
        Control::Dropdown(dropdown) if dropdown.id == *id => Some(dropdown),
        Control::Dropdown(_)
        | Control::Button(_)
        | Control::Toggle(_)
        | Control::Swatches(_)
        | Control::Stepper(_)
        | Control::Field(_)
        | Control::Choices(_)
        | Control::Separator => None,
    })
}

/// The dropdown named `id` with the row it is in: the toolbar, or the dock.
fn open_dropdown<'a>(
    id: &ControlId,
    toolbar: (&'a ToolbarModel, &'a Panel),
    dock: Option<(&'a ControlsModel, &'a Panel)>,
) -> Option<(&'a Dropdown, &'a Panel)> {
    let in_toolbar = (toolbar.0.sections.iter()).find_map(|section| match section {
        ToolbarSection::Zoom(dropdown) if dropdown.id == *id => Some((dropdown, toolbar.1)),
        ToolbarSection::Zoom(_) | ToolbarSection::Tools(_) => None,
    });
    in_toolbar.or_else(|| {
        let (model, row) = dock?;
        Some((holds(&model.controls, id)?, row))
    })
}

thread_local! {
    static BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times this thread has laid the panels out from the models, for
/// a test that pins how often one event does.
pub fn layout_builds() -> u64 {
    BUILDS.with(std::cell::Cell::get)
}

/// The built-in panels for `app` as it is now, in logical screen pixels.
///
/// The layout is shared with the cache, which lays it out again only when
/// something it is made from has changed. See [`cache`](self::cache).
pub fn layout(app: &App) -> Arc<PanelLayout> {
    field::overlay(app, cache::base(app))
}

/// The panels laid out afresh, with the cache left alone: what [`layout`]
/// must always equal.
#[doc(hidden)]
pub fn layout_uncached(app: &App) -> PanelLayout {
    let mut panels = build(app);
    field::overlay_in_place(app, &mut panels);
    panels
}

/// The panels laid out from the models alone, with no field showing an edit.
/// The editor asks where a field is while it lays the field's text out, so
/// that answer cannot depend on the editor.
fn build(app: &App) -> PanelLayout {
    let ui = &app.session.panel;
    if !ui.built_in {
        return PanelLayout::default();
    }
    BUILDS.with(|builds| builds.set(builds.get() + 1));
    let ctx = Ctx { app, ui };
    let viewport = app.session.viewport;
    let menu = (ui.menu.as_ref())
        .and_then(|open| Some((context::model(&ctx, open)?, open.at)))
        .and_then(|(model, at)| context::layout(&ctx, &model, at, viewport));
    if ui.menu_only {
        return PanelLayout {
            dropdown: menu,
            ..PanelLayout::default()
        };
    }
    let toolbar_model = super::toolbar(app);
    let dock_model = super::dock(app);
    let toolbar = toolbar::layout(&ctx, &toolbar_model, viewport);
    let dock = dock::layout(&ctx, dock_model.as_ref(), viewport);
    let dropdown = ui.open.as_ref().and_then(|id| {
        let (model, row) = open_dropdown(
            id,
            (&toolbar_model, &toolbar),
            dock_model.as_ref().map(|model| (model, &dock)),
        )?;
        let trigger = (row.nodes.iter()).find(|node| node.id.as_ref() == Some(id))?;
        // A list hangs from the bottom edge of the row, not from the
        // control inset in it.
        let hang = row.rect.bottom();
        Some(dropdown::layout(&ctx, model, trigger.rect, hang, viewport))
    });
    let (sidebar, sidebar_list) = sidebar_panels(&ctx, viewport);
    PanelLayout {
        sidebar,
        sidebar_list,
        tabs: Some(strip::layout(&ctx, &super::view_strip(app), viewport)),
        toolbar: Some(toolbar),
        dock: Some(dock),
        dropdown: menu.or(dropdown),
    }
}

/// The sidebar's panels while it is shown.
fn sidebar_panels(ctx: &Ctx<'_>, viewport: Vec2) -> (Option<Panel>, Option<Panel>) {
    if !ctx.app.session.sidebar.shown() {
        return (None, None);
    }
    let model = crate::sidebar(ctx.app);
    let built = sidebar::layout(ctx, &model, viewport);
    (Some(built.frame), Some(built.list))
}
