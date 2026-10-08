//! What is under a panel's controls: the toolbar's strip, a floating
//! frame.

use specular_interact::panel::builtin::{Panel, Surface};

use super::colors::{
    MENU, MENU_BORDER, MENU_SHADOW, POPUP, POPUP_BORDER, SHADOW_FAR, SHADOW_NEAR, SIDEBAR, TOOLBAR,
    TOOLBAR_BORDER,
};
use crate::{Item, Rect, RectDraw, ShadowDraw, Stroke, StrokeAlign};

/// The corner of a floating panel: `rounded-[10px]` in
/// `POPUP_SURFACE_CLASS`.
const RADIUS: f32 = 10.0;

pub(super) fn draw(panel: &Panel, out: &mut Vec<Item>) {
    let rect = super::rect(panel.rect);
    match panel.surface {
        Surface::Toolbar => {
            out.push(Item::screen(RectDraw::filled(rect, TOOLBAR)));
            let line = Rect::new(rect.x, rect.bottom() - 1.0, rect.width, 1.0);
            out.push(Item::screen(RectDraw::filled(line, TOOLBAR_BORDER)));
        }
        Surface::Sidebar => {
            out.push(Item::screen(RectDraw::filled(rect, SIDEBAR)));
            let edge = Rect::new(rect.right() - 1.0, rect.y, 1.0, rect.height);
            out.push(Item::screen(RectDraw::filled(edge, POPUP_BORDER)));
        }
        Surface::SidebarList => {}
        Surface::Popup | Surface::Dropdown if panel.menu => menu(rect, out),
        Surface::Popup | Surface::Dropdown => floating(rect, out),
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

fn floating(rect: Rect, out: &mut Vec<Item>) {
    out.push(shadow(rect, 4.0, 0.0, 16.0, SHADOW_FAR));
    out.push(shadow(rect, 10.0, -6.0, 8.0, SHADOW_NEAR));
    let frame = RectDraw::filled(rect, POPUP)
        .with_corner_radius(RADIUS)
        .with_stroke(Stroke::new(POPUP_BORDER, 1.0, StrokeAlign::Inside));
    out.push(Item::screen(frame));
}

/// `shadow-xl` on a white box with a zinc edge, which `TextSizeDropdown`
/// uses instead of the shared popup surface.
fn menu(rect: Rect, out: &mut Vec<Item>) {
    out.push(shadow(rect, 20.0, -5.0, 25.0, MENU_SHADOW));
    out.push(shadow(rect, 8.0, -6.0, 10.0, MENU_SHADOW));
    let frame = RectDraw::filled(rect, MENU)
        .with_corner_radius(RADIUS)
        .with_stroke(Stroke::new(MENU_BORDER, 1.0, StrokeAlign::Inside));
    out.push(Item::screen(frame));
}
