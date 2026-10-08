//! What is under a panel's controls: the toolbar's strip, a floating
//! frame.

use specular_interact::panel::builtin::{Panel, Surface};

use crate::{Item, PanelColors, Rect, RectDraw, ShadowDraw, Stroke, StrokeAlign};

/// The corner of a floating panel: `rounded-[10px]` in
/// `POPUP_SURFACE_CLASS`.
const RADIUS: f32 = 10.0;

pub(super) fn draw(p: &PanelColors, panel: &Panel, out: &mut Vec<Item>) {
    let rect = super::rect(panel.rect);
    match panel.surface {
        Surface::Toolbar => {
            out.push(Item::screen(RectDraw::filled(rect, p.toolbar)));
            let line = Rect::new(rect.x, rect.bottom() - 1.0, rect.width, 1.0);
            out.push(Item::screen(RectDraw::filled(line, p.toolbar_border)));
        }
        Surface::Sidebar => {
            out.push(Item::screen(RectDraw::filled(rect, p.sidebar)));
            let edge = Rect::new(rect.right() - 1.0, rect.y, 1.0, rect.height);
            out.push(Item::screen(RectDraw::filled(edge, p.popup_border)));
        }
        Surface::SidebarList => {}
        Surface::Popup | Surface::Dropdown if panel.menu => menu(p, rect, out),
        Surface::Popup | Surface::Dropdown => floating(p, rect, out),
    }
}

/// A CSS `box-shadow` of `rect`: moved down by `drop`, grown by `spread`
/// (shrunk when negative) and blurred.
fn shadow(rect: Rect, drop: f32, spread: f32, blur: f32, color: crate::Color) -> Item {
    Item::screen(ShadowDraw {
        rect: Rect::new(
            rect.x - spread,
            rect.y - spread + drop,
            rect.width + spread * 2.0,
            rect.height + spread * 2.0,
        ),
        corner_radius: (RADIUS + spread).max(0.0),
        blur,
        color,
    })
}

fn floating(p: &PanelColors, rect: Rect, out: &mut Vec<Item>) {
    out.push(shadow(rect, 4.0, 0.0, 16.0, p.shadow_far));
    out.push(shadow(rect, 10.0, -6.0, 8.0, p.shadow_near));
    let frame = RectDraw::filled(rect, p.popup)
        .with_corner_radius(RADIUS)
        .with_stroke(Stroke::new(p.popup_border, 1.0, StrokeAlign::Inside));
    out.push(Item::screen(frame));
}

/// `shadow-xl` on a white box with a zinc edge, which `TextSizeDropdown`
/// uses instead of the shared popup surface.
fn menu(p: &PanelColors, rect: Rect, out: &mut Vec<Item>) {
    out.push(shadow(rect, 20.0, -5.0, 25.0, p.menu_shadow));
    out.push(shadow(rect, 8.0, -6.0, 10.0, p.menu_shadow));
    let frame = RectDraw::filled(rect, p.menu)
        .with_corner_radius(RADIUS)
        .with_stroke(Stroke::new(p.menu_border, 1.0, StrokeAlign::Inside));
    out.push(Item::screen(frame));
}
