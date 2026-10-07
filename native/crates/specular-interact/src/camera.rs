//! Wheel and pinch: scroll the focused page, or pan and zoom the canvas.
//!
//! Wheel over the focused page scrolls that page. Anywhere else it pans the
//! canvas, and Cmd/Ctrl+wheel or a pinch zooms about the pointer.

use specular_core::{InputEvent, ViewportInputDelta, WheelEvent};

use crate::{App, Effect, WheelInput, hit};

pub(crate) fn on_wheel(app: &mut App, input: &WheelInput, effects: &mut Vec<Effect>) {
    let zooming = input.modifiers.meta || input.modifiers.control;
    let session = &app.session;
    if !zooming
        && let Some(screen) = session.pointer
        && let world = session.camera.screen_to_world(screen).as_dvec2()
        && let Some((page, placement)) = hit::page_at(app, world)
        && session.focus.page() == Some(&page)
    {
        let per_css = placement.canvas_per_css().as_vec2();
        effects.push(Effect::ForwardInput {
            page,
            event: InputEvent::Wheel(WheelEvent {
                position: placement.page_local(world).as_vec2(),
                delta: input.delta / session.camera.zoom / per_css,
                modifiers: input.modifiers,
            }),
        });
        return;
    }
    let delta = if zooming {
        ViewportInputDelta {
            zoom_delta_y: -input.delta.y,
            anchor: session.pointer,
            ..ViewportInputDelta::default()
        }
    } else {
        ViewportInputDelta {
            pan: input.delta,
            ..ViewportInputDelta::default()
        }
    };
    app.session.camera.apply_input_delta(delta);
}

/// Zooms about the pointer, or the viewport centre when the pointer is
/// outside the window.
pub(crate) fn on_pinch(app: &mut App, delta: f32) {
    let session = &mut app.session;
    let anchor = session.pointer.unwrap_or(session.viewport / 2.0);
    session
        .camera
        .zoom_about(anchor, session.camera.zoom * (1.0 + delta));
}
