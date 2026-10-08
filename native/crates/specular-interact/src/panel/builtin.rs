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

mod controls;
mod dropdown;
mod metrics;
mod node;
mod place;
mod popup;
mod route;
mod toolbar;
mod trigger;

use glam::Vec2;

pub use self::metrics::TOOLBAR_HEIGHT;
pub use self::node::{
    Chrome, Node, NodeState, Panel, PanelRect, Part, Pointing, Surface, Tint, Tone,
};
pub(crate) use self::route::{cancel, hit, on_pointer, over, swallows_scroll, tidy};
use super::{Control, ControlId, Dropdown, PopupAnchor, PopupModel, ToolbarModel, ToolbarSection};
use crate::App;

/// What the built-in panels remember between events.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
}

/// The panels as laid out for one frame. Empty when the built-in panels are
/// off.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelLayout {
    /// The toolbar, a strip across the top of the viewport.
    pub toolbar: Option<Panel>,
    /// The popup of the tool in hand or of the selection.
    pub popup: Option<Panel>,
    /// The list under the open dropdown.
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
        [&self.toolbar, &self.popup, &self.dropdown]
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
    /// How a control is drawn now. A control that cannot be used takes no
    /// hover or press.
    fn state(&self, id: &ControlId, enabled: bool, on: bool) -> NodeState {
        let over = self.ui.hover.as_ref() == Some(id);
        let held = self.ui.pressed.as_ref() == Some(id);
        let pointing = match (enabled, over, held) {
            (true, true, true) => Pointing::Pressed,
            (true, true, false) => Pointing::Hover,
            (false, ..) | (true, false, _) => Pointing::Away,
        };
        NodeState {
            enabled,
            on,
            pointing,
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
            | Control::Separator => None,
        })
    })
}

/// The built-in panels for `app` as it is now, in logical screen pixels.
pub fn layout(app: &App) -> PanelLayout {
    let ui = &app.session.panel;
    if !ui.built_in {
        return PanelLayout::default();
    }
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
            Surface::Popup | Surface::Dropdown => popup.as_ref(),
        }?;
        let trigger = host
            .nodes
            .iter()
            .find(|node| node.id.as_ref() == Some(id))?;
        // A list of the toolbar hangs from the strip's bottom edge, not from
        // the button inset in it.
        let hang = match surface {
            Surface::Toolbar => host.rect.bottom(),
            Surface::Popup | Surface::Dropdown => trigger.rect.bottom(),
        };
        Some(dropdown::layout(&ctx, model, trigger.rect, hang, viewport))
    });
    PanelLayout {
        toolbar,
        popup,
        dropdown,
    }
}
