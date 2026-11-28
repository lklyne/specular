//! Overlays a mod opens (ADR 0059): a copy of a mod over the window, as a
//! dialog, a popup, a drawer or a window, until it is closed.
//!
//! They are app state, not the window's, so a headless run and the API see
//! what is open (the `overlays` key) and Escape closes the top dialog or
//! popup through `update` like any other key. The window draws each one as
//! its `kind` says.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Action, App, ProjectId};

/// The project card's width, as the sidebar mod opens it.
const CARD_WIDTH: f32 = 300.0;

/// How an overlay sits over the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlayKind {
    /// Centred over a scrim; Escape closes it.
    #[default]
    Dialog,
    /// At a point; a press outside it or Escape closes it.
    Popup,
    /// Along an edge of the window, its full length.
    Drawer,
    /// Stays until it is closed, where it was put.
    Window,
}

impl OverlayKind {
    /// Whether Escape closes it.
    pub const fn escapes(self) -> bool {
        matches!(self, Self::Dialog | Self::Popup | Self::Drawer)
    }
}

/// One open overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlaySpec {
    /// Its name: the copy's id, which the mod reads as `props().overlay`.
    /// The mod's name when not given.
    #[serde(default)]
    pub id: String,
    /// The mod it is a copy of.
    #[serde(rename = "mod")]
    pub name: String,
    /// What the copy is handed, with `overlay` beside.
    #[serde(default)]
    pub props: Map<String, Value>,
    /// How it sits.
    #[serde(default)]
    pub kind: OverlayKind,
    /// Its width in points, else the content's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// Its height in points, else the content's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    /// A popup's or window's corner, in window points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<[f32; 2]>,
    /// A drawer's edge: `left`, `right`, `top` or `bottom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge: Option<String>,
    /// `none` leaves the frame to the mod: no panel, edge or shadow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
}

impl OverlaySpec {
    /// Reads `overlay.open`'s arguments.
    pub fn parse(args: &Value) -> Result<Self, String> {
        let mut spec: Self = serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
        if spec.name.is_empty() {
            return Err("`mod` names the mod to open".to_owned());
        }
        if spec.id.is_empty() {
            spec.id.clone_from(&spec.name);
        }
        Ok(spec)
    }

    /// The props the copy is handed: its own, and `overlay`, its id.
    pub fn copy_props(&self) -> Value {
        let mut props = self.props.clone();
        props.insert("overlay".to_owned(), Value::String(self.id.clone()));
        Value::Object(props)
    }

    /// Whether the window draws its frame.
    pub fn framed(&self) -> bool {
        self.frame.as_deref() != Some("none")
    }
}

/// What is open, bottom first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Overlays {
    open: Vec<OverlaySpec>,
}

impl Overlays {
    /// What is open, bottom first.
    pub fn open(&self) -> &[OverlaySpec] {
        &self.open
    }
}

impl App {
    /// The mod overlays open, bottom first.
    pub fn overlays(&self) -> &[OverlaySpec] {
        self.session.overlays.open()
    }
}

/// A change to what is open.
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayAction {
    /// Open one, or move one open by that id to the top with its new spec.
    Open(Box<OverlaySpec>),
    /// Close the one named, or the top one.
    Close(Option<String>),
}

/// Opens the card of the project `id` (the `project` mod) as a popup. The
/// sidebar mod opens it beside the project's head; by a name or from the
/// palette it sits at the window's corner.
pub(crate) fn project_card(id: &ProjectId) -> Action {
    let mut props = Map::new();
    props.insert("project".to_owned(), Value::from(id.as_str()));
    Action::Overlay(OverlayAction::Open(Box::new(OverlaySpec {
        id: "project".to_owned(),
        name: "project".to_owned(),
        props,
        kind: OverlayKind::Popup,
        width: Some(CARD_WIDTH),
        height: None,
        at: None,
        edge: None,
        frame: None,
    })))
}

/// Runs `action`.
pub(crate) fn run(app: &mut App, action: OverlayAction) {
    let open = &mut app.session.overlays.open;
    match action {
        OverlayAction::Open(spec) => {
            open.retain(|overlay| overlay.id != spec.id);
            open.push(*spec);
        }
        OverlayAction::Close(Some(id)) => open.retain(|overlay| overlay.id != id),
        OverlayAction::Close(None) => {
            open.pop();
        }
    }
}

/// Escape closes the top overlay when Escape closes it. Returns whether it
/// did.
pub(crate) fn escape(app: &mut App) -> bool {
    let open = &mut app.session.overlays.open;
    if open.last().is_some_and(|top| top.kind.escapes()) {
        open.pop();
        return true;
    }
    false
}

/// The `overlays` key: what is open, bottom first.
pub(crate) fn overlays_json(app: &App) -> Value {
    serde_json::to_value(app.session.overlays.open()).unwrap_or(Value::Null)
}
