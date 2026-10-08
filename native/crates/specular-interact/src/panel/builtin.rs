//! The toolbar and the item popup laid out, hit-tested and clicked with no
//! UI library: the stand-in for a panel renderer.
//!
//! [`layout`] turns the two models into rects in logical screen pixels, each
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
mod dropdown;
mod field;
mod metrics;
mod node;
mod place;
mod popup;
mod route;
mod scroll;
mod sidebar;
mod toolbar;
mod trigger;

use std::sync::Arc;

use glam::Vec2;

pub use self::cache::LayoutCache;
pub(crate) use self::cache::{
    forget as forget_layout, forget_unless as forget_layout_unless, keeps_layout,
};
pub use self::context::ContextMenu;
pub(crate) use self::context::open as open_menu;
pub(crate) use self::field::{field_box, field_text_area};
pub use self::metrics::{FIELD_HEIGHT, FIELD_LINE, FIELD_TEXT, TOOLBAR_HEIGHT};
pub use self::node::{
    Chrome, Input, InputFocus, Node, NodeState, Panel, PanelRect, Part, Pointing, Surface, Tint,
    Tone,
};
pub(crate) use self::route::{cancel, hit, on_pointer, over, over_field, swallows_scroll, tidy};
pub(crate) use self::scroll::on_wheel;
use super::{Control, ControlId, Dropdown, PopupAnchor, PopupModel, ToolbarModel, ToolbarSection};
use crate::App;

/// What the built-in panels remember between events.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelUi {
    /// Whether the built-in panels are shown, hit and clicked at all.
    pub built_in: bool,
    /// Whether only the popups beside a canvas item are built in: the
    /// toolbar and a popup hung from it are someone else's to draw.
    pub canvas_only: bool,
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
    /// Only the popups beside a canvas item, with nothing open or hovered.
    pub fn canvas_popups() -> Self {
        Self {
            built_in: true,
            canvas_only: true,
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
    /// The toolbar, a strip across the top of the viewport.
    pub toolbar: Option<Panel>,
    /// The popup of the tool in hand or of the selection.
    pub popup: Option<Panel>,
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
            &self.toolbar,
            &self.popup,
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
            font,
            size: metrics::TEXT_SIZE,
            line_height: metrics::TEXT_LINE,
            wrap_width: None,
            align: specular_doc::TextAlign::Left,
        };
        let layout = self.app.text_measure().layout(text, &spec);
        let width = layout.lines.first().map_or(0.0, |line| {
            let start = line.stops.first().map_or(0.0, |stop| stop.x);
            line.stops.last().map_or(0.0, |stop| stop.x) - start
        });
        width.ceil()
    }
}

/// The dropdown named `id`, in the toolbar or at the top level of the popup.
fn open_dropdown<'a>(
    id: &ControlId,
    toolbar: &'a ToolbarModel,
    popup: Option<&'a PopupModel>,
) -> Option<(&'a Dropdown, Surface)> {
    let in_toolbar = toolbar.sections.iter().find_map(|section| match section {
        ToolbarSection::Zoom(dropdown) if dropdown.id == *id => Some((dropdown, Surface::Toolbar)),
        ToolbarSection::Zoom(_) | ToolbarSection::Tools(_) => None,
    });
    in_toolbar.or_else(|| {
        popup?.controls.iter().find_map(|control| match control {
            Control::Dropdown(dropdown) if dropdown.id == *id => Some((dropdown, Surface::Popup)),
            Control::Dropdown(_)
            | Control::Button(_)
            | Control::Toggle(_)
            | Control::Swatches(_)
            | Control::Stepper(_)
            | Control::Field(_)
            | Control::Choices(_)
            | Control::Separator => None,
        })
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
    let toolbar_model = super::toolbar(app);
    let popup_model = super::popup_for(app);
    let toolbar = (!ui.canvas_only).then(|| toolbar::layout(&ctx, &toolbar_model, viewport));
    let popup = popup_model
        .as_ref()
        .filter(|model| !ui.canvas_only || matches!(model.anchor, PopupAnchor::Canvas { .. }))
        .and_then(|model| popup::layout(&ctx, model, viewport));
    let dropdown = ui.open.as_ref().and_then(|id| {
        let (model, surface) = open_dropdown(id, &toolbar_model, popup_model.as_ref())?;
        let host = match surface {
            Surface::Toolbar => toolbar.as_ref(),
            Surface::Popup | Surface::Dropdown | Surface::Sidebar | Surface::SidebarList => {
                popup.as_ref()
            }
        }?;
        let trigger = host
            .nodes
            .iter()
            .find(|node| node.id.as_ref() == Some(id))?;
        // A list of the toolbar hangs from the strip's bottom edge, not from
        // the button inset in it.
        let hang = match surface {
            Surface::Toolbar => host.rect.bottom(),
            Surface::Popup | Surface::Dropdown | Surface::Sidebar | Surface::SidebarList => {
                trigger.rect.bottom()
            }
        };
        Some(dropdown::layout(&ctx, model, trigger.rect, hang, viewport))
    });
    let (sidebar, sidebar_list) = sidebar_panels(&ctx, viewport);
    let menu = (ui.menu.as_ref())
        .and_then(|open| context::model(&ctx, open))
        .and_then(|model| context::layout(&ctx, &model, viewport));
    PanelLayout {
        sidebar,
        sidebar_list,
        toolbar,
        popup,
        dropdown: menu.or(dropdown),
    }
}

/// The sidebar's panels while it is shown.
fn sidebar_panels(ctx: &Ctx<'_>, viewport: Vec2) -> (Option<Panel>, Option<Panel>) {
    // A shell that draws its own sidebar keeps only the canvas popups.
    if ctx.ui.canvas_only || !ctx.app.session.sidebar.shown() {
        return (None, None);
    }
    let model = crate::sidebar(ctx.app);
    let built = sidebar::layout(ctx, &model, viewport);
    (Some(built.frame), Some(built.list))
}
