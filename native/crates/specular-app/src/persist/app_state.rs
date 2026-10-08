//! The camera in a `.canvas` file's `appState` (see `docs/file-formats.md`).

use glam::Vec2;
use serde_json::{Value, json};
use specular_core::Camera;
use specular_doc::{CanvasError, Document};

const APP_STATE: &str = "appState";

/// The camera `document` was saved with, or `None` when its `appState` has
/// no usable zoom and pan.
pub(crate) fn camera_of(document: &Document) -> Option<Camera> {
    let state = document.extra().get(APP_STATE)?;
    let number = |value: &Value| value.as_f64().filter(|number| number.is_finite());
    let zoom = number(state.get("zoom")?)?;
    let pan = state.get("pan")?;
    let (x, y) = (number(pan.get("x")?)?, number(pan.get("y")?)?);
    (zoom > 0.0).then(|| Camera::new(Vec2::new(x as f32, y as f32), zoom as f32))
}

/// The `.canvas` text of `document` with `camera` in its `appState`. The
/// rest of `appState` is kept as it was read.
pub(crate) fn canvas_text(document: &Document, camera: Camera) -> Result<String, CanvasError> {
    let mut document = document.clone();
    let state = document
        .extra_mut()
        .entry(APP_STATE)
        .or_insert_with(|| json!({}));
    if !state.is_object() {
        *state = json!({});
    }
    state["zoom"] = json!(widened(camera.zoom));
    state["pan"] = json!({ "x": widened(camera.pan.x), "y": widened(camera.pan.y) });
    document.to_canvas_string()
}

/// `value` as the `f64` that prints the same digits. A plain widening
/// keeps the `f32`'s rounding error, and 0.8 would be saved as
/// 0.800000011920929.
fn widened(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(f64::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(json: &str) -> Document {
        Document::from_canvas_str(json).unwrap()
    }

    #[test]
    fn the_camera_is_read_from_app_state() {
        let document = document(
            r#"{"nodes":[],"edges":[],"appState":{"zoom":0.5,"pan":{"x":-200,"y":-100}}}"#,
        );
        let camera = camera_of(&document).unwrap();
        assert_eq!((camera.zoom, camera.pan), (0.5, Vec2::new(-200.0, -100.0)));
        // A zoom outside the canvas range is clamped.
        let far =
            self::document(r#"{"nodes":[],"edges":[],"appState":{"zoom":40,"pan":{"x":0,"y":0}}}"#);
        assert_eq!(camera_of(&far).unwrap().zoom, 3.0);
    }

    #[test]
    fn a_missing_or_broken_app_state_gives_no_camera() {
        for json in [
            r#"{"nodes":[],"edges":[]}"#,
            r#"{"nodes":[],"edges":[],"appState":{"zoom":1}}"#,
            r#"{"nodes":[],"edges":[],"appState":{"zoom":"big","pan":{"x":0,"y":0}}}"#,
            r#"{"nodes":[],"edges":[],"appState":{"zoom":0,"pan":{"x":0,"y":0}}}"#,
            r#"{"nodes":[],"edges":[],"appState":7}"#,
            r#"{"nodes":[],"edges":[],"appState":{"pan":{"x":0,"y":0}}}"#,
            r#"{"nodes":[],"edges":[],"appState":{"zoom":1,"pan":{"x":0}}}"#,
        ] {
            assert_eq!(camera_of(&document(json)), None, "{json}");
        }
    }

    #[test]
    fn saving_writes_the_camera_and_keeps_the_rest_of_app_state() {
        let document = document(
            r#"{"nodes":[],"edges":[],"appState":{"zoom":1,"pan":{"x":0,"y":0},"leftSidebarOpen":true}}"#,
        );
        let camera = Camera::new(Vec2::new(12.5, -40.0), 0.75);
        let saved = self::document(&canvas_text(&document, camera).unwrap());
        assert_eq!(camera_of(&saved), Some(camera));
        assert_eq!(
            saved.extra()[APP_STATE]["leftSidebarOpen"],
            Value::Bool(true)
        );
    }

    #[test]
    fn the_zoom_is_written_as_the_number_it_is() {
        let camera = Camera::new(Vec2::new(-168.0, -518.0), 0.8);
        let text = canvas_text(&document(r#"{"nodes":[],"edges":[]}"#), camera).unwrap();
        // Not 0.800000011920929, the `f32` widened.
        assert!(text.contains(r#""zoom": 0.8,"#), "{text}");
    }

    #[test]
    fn saving_a_file_without_app_state_adds_one() {
        let camera = Camera::new(Vec2::new(3.0, 4.0), 2.0);
        let bare = document(r#"{"nodes":[],"edges":[]}"#);
        let text = canvas_text(&bare, camera).unwrap();
        assert_eq!(camera_of(&document(&text)), Some(camera));
        // The document it was given is left alone.
        assert!(bare.extra().get(APP_STATE).is_none());
        // An appState that is not an object is replaced by one.
        let broken = document(r#"{"nodes":[],"edges":[],"appState":7}"#);
        let text = canvas_text(&broken, camera).unwrap();
        assert_eq!(camera_of(&document(&text)), Some(camera));
    }
}
