//! The mod overlays (ADR 0059), over everything else: each open copy drawn
//! as its kind says. A dialog sits in the middle over a scrim, a popup at
//! its point over a clear layer that a press closes it through, a drawer
//! along its edge, and a window where it was put.
//!
//! Escape from inside one, where a mod's field has the keys and the canvas
//! does not, closes it here: the key comes up through the overlay. With
//! the canvas holding the keys, `update` closes it the same way.

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, InteractiveElement as _, IntoElement as _, KeyDownEvent, MouseButton,
    ParentElement as _, Styled as _, div, px,
};
use specular_interact::{Action, Event, OverlayAction, OverlayKind, OverlaySpec};

use crate::canvas;
use crate::theme;

/// The room a framed overlay keeps from the window's edges.
const MARGIN: f32 = 12.0;

/// How far down a dialog at the top sits.
const TOP: f32 = 96.0;

fn close(id: &str) {
    let action = OverlayAction::Close(Some(id.to_owned()));
    canvas::dispatch(Event::Action(Action::Overlay(action)));
}

/// The panel a framed overlay sits in; a frameless one is the mod's alone.
fn framed(spec: &OverlaySpec, content: AnyElement) -> gpui_kit::Div {
    let body = div().flex().flex_col().overflow_hidden().child(content);
    let body = body
        .when_some(spec.width, |body, width| body.w(px(width)))
        .when_some(spec.height, |body, height| body.h(px(height)));
    if !spec.framed() {
        return body;
    }
    body.rounded(px(crate::look::radius_large()))
        .bg(theme::solid(theme::popup()))
        .border_1()
        .border_color(theme::solid(theme::chrome_border()))
        .shadow_lg()
}

/// Where the panel goes for its kind, inside a layer over the window.
fn placed(spec: &OverlaySpec, panel: gpui_kit::Div, window: (f32, f32)) -> AnyElement {
    let layer = div().absolute().inset_0();
    let [x, y] = spec.at.unwrap_or([MARGIN, MARGIN]);
    let at = |panel: gpui_kit::Div| {
        panel
            .absolute()
            .left(px(x.clamp(0.0, (window.0 - MARGIN).max(0.0))))
            .top(px(y.clamp(0.0, (window.1 - MARGIN).max(0.0))))
    };
    match spec.kind {
        // `edge: top` keeps a dialog near the top, as a palette sits.
        OverlayKind::Dialog => layer
            .flex()
            .justify_center()
            .map(|layer| match spec.edge.as_deref() {
                Some("top") => layer.items_start().pt(px(TOP)),
                _ => layer.items_center(),
            })
            .bg(theme::tinted(theme::scrim()))
            .child(panel)
            .into_any_element(),
        OverlayKind::Popup => layer.child(at(panel)).into_any_element(),
        OverlayKind::Window => at(panel).into_any_element(),
        OverlayKind::Drawer => {
            let edge = spec.edge.as_deref().unwrap_or("right");
            let panel = panel.absolute();
            let panel = match edge {
                "left" => panel.left_0().top_0().bottom_0(),
                "top" => panel.top_0().left_0().right_0(),
                "bottom" => panel.bottom_0().left_0().right_0(),
                _ => panel.right_0().top_0().bottom_0(),
            };
            layer.child(panel).into_any_element()
        }
    }
}

/// One open overlay, or `None` while its copy has not loaded.
fn overlay(spec: &OverlaySpec, content: AnyElement, window: (f32, f32)) -> AnyElement {
    let id = spec.id.clone();
    let escapes = spec.kind.escapes();
    let panel = framed(spec, content)
        .id(gpui_kit::SharedString::from(format!("overlay-{}", spec.id)))
        .occlude()
        .on_key_down(move |event: &KeyDownEvent, _, cx| {
            if escapes && event.keystroke.key == "escape" {
                close(&id);
                cx.stop_propagation();
            }
        });
    let panel = div().child(panel);
    let placed = placed(spec, panel, window);
    match spec.kind {
        OverlayKind::Window => placed,
        // A press on the layer around the panel, not on the panel, closes
        // a popup; a dialog's scrim and a drawer's layer only stop it.
        OverlayKind::Popup | OverlayKind::Dialog | OverlayKind::Drawer => {
            let id = spec.id.clone();
            let closes = spec.kind == OverlayKind::Popup;
            div()
                .id(gpui_kit::SharedString::from(format!(
                    "overlay-layer-{}",
                    spec.id
                )))
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(MouseButton::Left, move |_, _, _| {
                    if closes {
                        close(&id);
                    }
                })
                .child(placed)
                .into_any_element()
        }
    }
}

/// Every open overlay whose copy has loaded, bottom first.
pub(super) fn layer(
    specs: &[OverlaySpec],
    copy: impl Fn(&str) -> Option<AnyElement>,
    window: (f32, f32),
) -> Vec<AnyElement> {
    (specs.iter())
        .filter_map(|spec| Some(overlay(spec, copy(&spec.id)?, window)))
        .collect()
}
