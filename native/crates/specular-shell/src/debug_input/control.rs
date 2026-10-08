//! Where a control is, by the name its model gives it.
//!
//! A control the Kit draws is where GPUI last laid it out. One the canvas
//! draws in its own pass (the popup beside an item, the context menu and
//! the lists they open) is where the app's built-in layout has it. One
//! name works for either.

use glam::Vec2;
use specular_interact::panel::builtin::layout;

use crate::canvas;
use crate::view::shown_controls;

/// The middle of the control named `name` among `kit`'s and `canvas`'s, or
/// an error listing the names that are shown. Both lists are in paint
/// order, and the Kit's are in front of the canvas's.
pub(super) fn find(
    name: &str,
    kit: &[(String, Vec2)],
    canvas: &[(String, Vec2)],
) -> Result<Vec2, String> {
    let among = |shown: &[(String, Vec2)]| {
        (shown.iter().rev())
            .find(|(shown, _)| shown == name)
            .map(|(_, at)| *at)
    };
    among(kit).or_else(|| among(canvas)).ok_or_else(|| {
        let mut names: Vec<&str> = Vec::new();
        for (shown, _) in kit.iter().chain(canvas) {
            if !names.contains(&shown.as_str()) {
                names.push(shown);
            }
        }
        format!(
            "no control `{name}` is shown; these are: {}",
            names.join(" ")
        )
    })
}

/// The controls the canvas draws itself, in points from the window's
/// content corner.
pub(super) fn in_canvas_pass() -> Vec<(String, Vec2)> {
    canvas::with(|canvas| {
        let origin = (canvas.runtime.window())
            .map_or(Vec2::ZERO, crate::surface::CanvasSurface::slot_origin);
        let layout = layout(canvas.runtime.app());
        (layout.panels())
            .flat_map(|panel| &panel.nodes)
            .filter_map(|node| Some((node.id.as_ref()?, node.rect.centre())))
            .map(|(id, centre)| (id.as_str().to_owned(), centre + origin))
            .collect()
    })
    .unwrap_or_default()
}

/// The middle of the control named `name` as the window shows it now.
pub(super) fn locate(name: &str) -> Result<Vec2, String> {
    find(name, &shown_controls(), &in_canvas_pass())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_found_wherever_it_is_drawn_and_one_not_shown_lists_those_that_are() {
        let at = |name: &str, x: f32| (name.to_owned(), Vec2::new(x, 22.0));
        let kit = [
            at("tool.select", 600.0),
            at("tool.draw", 640.0),
            at("tool.draw", 650.0),
        ];
        let canvas = [at("shape.color", 900.0), at("tool.select", 1.0)];
        assert_eq!(
            find("tool.select", &kit, &canvas),
            Ok(Vec2::new(600.0, 22.0))
        );
        // Drawn twice, the one in front takes the click.
        assert_eq!(find("tool.draw", &kit, &canvas), Ok(Vec2::new(650.0, 22.0)));
        assert_eq!(
            find("shape.color", &kit, &canvas),
            Ok(Vec2::new(900.0, 22.0))
        );
        assert_eq!(
            find("shape.kind", &kit, &canvas),
            Err(
                "no control `shape.kind` is shown; these are: tool.select tool.draw shape.color"
                    .to_owned()
            )
        );
    }
}
