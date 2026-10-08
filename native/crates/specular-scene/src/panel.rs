//! The built-in toolbar and item popup as scene items.
//!
//! [`draw_panels`] paints what
//! [`layout`](specular_interact::panel::builtin::layout) laid out: every
//! rect and every glyph's box comes from there, and this module only
//! chooses colors and turns glyphs into paths. All of it is in screen space
//! and goes on top of whatever the scene already holds.
//!
//! [`view`](crate::view) does not call this. A shell that wants the
//! built-in panels draws them over the scene it got from `view`; one that
//! draws the panel models with a UI library leaves this module out.

mod colors;
mod icons;
mod node;
mod surface;

use specular_interact::App;
use specular_interact::panel::builtin::{PanelRect, layout};

use crate::{Rect, Scene};

fn rect(rect: PanelRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width, rect.height)
}

/// Adds the built-in panels of `app` to `scene`, over everything in it: the
/// toolbar, then the popup, then the list of the open dropdown. Nothing is
/// added while the built-in panels are off.
pub fn draw_panels(app: &App, scene: &mut Scene) {
    let layout = layout(app);
    for panel in layout.panels() {
        surface::draw(panel, &mut scene.items);
        for node in &panel.nodes {
            node::draw(panel.surface, node, &mut scene.items);
        }
    }
}
