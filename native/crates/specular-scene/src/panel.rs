//! The built-in chrome, sidebar and lists as scene items.
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

mod icons;
mod input;
mod node;
mod surface;

use specular_interact::App;
use specular_interact::panel::builtin::{PanelRect, Surface, layout};

use crate::{Colors, Rect, Scene};

fn rect(rect: PanelRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width, rect.height)
}

pub use self::icons::icon_svg;

/// The color a panel paints `color` in: the stored color resolved against
/// the hues of `palette` for how `role` uses it. The same resolution the
/// built-in swatches and tinted glyphs get.
pub fn panel_color(
    color: &specular_doc::Color,
    palette: specular_interact::Palette,
    role: specular_interact::PaintRole,
    appearance: specular_interact::Appearance,
) -> crate::Color {
    use crate::view::palette;
    let hues = match palette {
        specular_interact::Palette::Soft => palette::Palette::Soft,
        specular_interact::Palette::Vivid => palette::Palette::Vivid,
    };
    let role = match role {
        specular_interact::PaintRole::Fill => palette::Role::Fill,
        specular_interact::PaintRole::Ink => palette::Role::Ink,
    };
    palette::resolve(color, hues, role, Colors::of(appearance))
}

/// Adds the built-in panels of `app` to `scene`, over everything in it: the
/// sidebar, then the rows of the chrome, then the list of the open dropdown. Nothing is
/// added while the built-in panels are off.
pub fn draw_panels(app: &App, scene: &mut Scene) {
    let layout = layout(app);
    let colors = Colors::of(app.appearance());
    for panel in layout.panels() {
        surface::draw(&colors.panel, panel, &mut scene.items);
        for node in &panel.nodes {
            let first = scene.items.len();
            node::draw(colors, panel.surface, node, &mut scene.items);
            if panel.surface == Surface::SidebarList {
                clip(&mut scene.items[first..], rect(panel.rect));
            }
        }
    }
}

/// Cuts `items` off at `window`, inside any clip they already have.
fn clip(items: &mut [crate::Item], window: Rect) {
    for item in items {
        item.clip = match item.clip {
            Some(own) => own.intersection(window),
            None => Some(window),
        }
        .or(Some(Rect::new(0.0, 0.0, 0.0, 0.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Item, RectDraw};

    #[test]
    fn a_window_cuts_items_inside_their_own_clip_and_hides_those_outside_it() {
        let card = || RectDraw::filled(Rect::new(0.0, 0.0, 400.0, 400.0), crate::Color::WHITE);
        let window = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut items = [
            Item::screen(card()),
            Item::screen(card()).clipped(Rect::new(50.0, 50.0, 100.0, 100.0)),
            Item::screen(card()).clipped(Rect::new(200.0, 200.0, 10.0, 10.0)),
        ];
        clip(&mut items, window);
        assert_eq!(
            items.map(|item| item.clip),
            [
                Some(window),
                Some(Rect::new(50.0, 50.0, 50.0, 50.0)),
                Some(Rect::new(0.0, 0.0, 0.0, 0.0)),
            ]
        );
    }
}
