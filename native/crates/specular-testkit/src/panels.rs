//! The built-in toolbar and popup in a test: turning them on, clicking a
//! control by its name, and the layout as text for `insta`.

use std::fmt::Write as _;

use glam::Vec2;
use specular_interact::panel::builtin::{
    Node, Panel, PanelLayout, PanelRect, Part, Pointing, Surface, layout,
};
use specular_interact::{ControlId, Event};
use specular_scene::Scene;

use crate::TestApp;

/// The viewport a test that never set one gets, the same one
/// [`TestApp::scene_snapshot`] assumes.
const VIEWPORT: Vec2 = Vec2::new(1600.0, 1000.0);

fn num(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    format!("{rounded}")
}

fn rect(rect: PanelRect) -> String {
    format!(
        "{},{} {}x{}",
        num(rect.x),
        num(rect.y),
        num(rect.width),
        num(rect.height)
    )
}

fn part(part: &Part) -> String {
    match part {
        Part::Icon {
            icon,
            rect: area,
            tint,
        } => {
            let tint = tint
                .as_ref()
                .map_or_else(String::new, |tint| format!(" tint={}", tint.color.as_str()));
            format!("icon {icon:?} {}{tint}", rect(*area))
        }
        Part::Text {
            text,
            rect: area,
            align,
            ..
        } => format!("text {text:?} {} {align:?}", rect(*area)),
        Part::Dot { rect: area, tint } => {
            let color = tint.as_ref().map_or("none", |tint| tint.color.as_str());
            format!("dot {color} {}", rect(*area))
        }
        Part::Chevron { rect: area } => format!("chevron {}", rect(*area)),
        Part::Check { rect: area } => format!("check {}", rect(*area)),
        Part::Key { text, rect: area } => format!("key {text:?} {}", rect(*area)),
    }
}

fn node(out: &mut String, node: &Node) {
    let name = node.id.as_ref().map_or("-", ControlId::as_str);
    let _ = write!(out, "  {name} {} {:?}", rect(node.rect), node.chrome);
    if node.state.on {
        out.push_str(" on");
    }
    if !node.state.enabled {
        out.push_str(" disabled");
    }
    match node.state.pointing {
        Pointing::Away => {}
        Pointing::Hover => out.push_str(" hover"),
        Pointing::Pressed => out.push_str(" pressed"),
    }
    let parts: Vec<String> = node.parts.iter().map(part).collect();
    if !parts.is_empty() {
        let _ = write!(out, ": {}", parts.join("; "));
    }
    out.push('\n');
}

fn panel(out: &mut String, panel: &Panel) {
    let name = match panel.surface {
        Surface::Toolbar => "toolbar",
        Surface::Popup => "popup",
        Surface::Dropdown => "dropdown",
    };
    let menu = if panel.menu { " menu" } else { "" };
    let _ = writeln!(out, "{name}{menu} {}", rect(panel.rect));
    for it in &panel.nodes {
        node(out, it);
    }
}

/// The built-in panels as stable text: each panel's box, then a line per
/// node with its box, what is painted behind it, its state and its parts.
pub fn layout_snapshot(layout: &PanelLayout) -> String {
    let mut out = String::new();
    for it in layout.panels() {
        panel(&mut out, it);
    }
    out.trim_end().to_owned()
}

impl TestApp {
    /// Turns the built-in toolbar and popup on, as a shell that draws them
    /// does at startup. A test that never set a viewport gets a 1600x1000
    /// one, since the panels are laid out against it.
    pub fn with_panels(&mut self) -> &mut Self {
        if self.session().viewport == Vec2::ZERO {
            self.viewport(VIEWPORT);
        }
        self.send(Event::BuiltinPanels(true))
    }

    /// The built-in panels as laid out now.
    pub fn panel_layout(&self) -> PanelLayout {
        layout(&self.app)
    }

    /// The box of the control named `id`.
    #[track_caller]
    pub fn control_rect(&self, id: &str) -> PanelRect {
        let layout = self.panel_layout();
        let Some(node) = layout.node(&ControlId::from(id.to_owned())) else {
            let shown: Vec<&str> = layout.controls().map(ControlId::as_str).collect();
            panic!(
                "no control {id:?} is shown; these are: {}",
                shown.join(", ")
            )
        };
        node.rect
    }

    /// Moves the pointer onto the control named `id`.
    #[track_caller]
    pub fn hover_control(&mut self, id: &str) -> &mut Self {
        let at = self.control_rect(id).centre();
        self.pointer_move(at)
    }

    /// Presses the control named `id` and keeps the button down.
    #[track_caller]
    pub fn press_control(&mut self, id: &str) -> &mut Self {
        let at = self.control_rect(id).centre();
        self.pointer_move(at).press(at)
    }

    /// Clicks the control named `id`, looking its box up in the layout.
    #[track_caller]
    pub fn click_control(&mut self, id: &str) -> &mut Self {
        self.press_control(id).release()
    }

    /// The built-in panels as stable text. See [`layout_snapshot`].
    pub fn panel_snapshot(&self) -> String {
        layout_snapshot(&self.panel_layout())
    }

    /// What the built-in panels draw, without the canvas under them, as
    /// stable text. See [`scene_snapshot`](crate::scene_snapshot).
    pub fn panel_scene_snapshot(&self) -> String {
        let mut scene = Scene::new();
        specular_scene::draw_panels(&self.app, &mut scene);
        crate::scene_snapshot(&scene)
    }
}

/// Asserts the layout of the built-in panels of a [`TestApp`] against an
/// inline snapshot.
#[macro_export]
macro_rules! assert_panel_snapshot {
    ($app:expr, $($rest:tt)*) => {
        $crate::insta::assert_snapshot!($app.panel_snapshot(), $($rest)*)
    };
}
