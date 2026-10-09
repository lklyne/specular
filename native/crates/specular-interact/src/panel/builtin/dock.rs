//! The dock: the third row of the chrome, with the controls of the tool in
//! hand or of the selection from its left end.

use glam::Vec2;

use super::super::PopupModel;
use super::controls::{self, RowKind};
use super::metrics::DOCK_PAD;
use super::node::{Panel, Surface};
use super::{Ctx, rows};

/// The dock across a viewport `viewport` wide, holding the controls of
/// `model`. With no model it is an empty bar. A control that stretches takes
/// the room the rest leave.
pub(super) fn layout(ctx: &Ctx<'_>, model: Option<&PopupModel>, viewport: Vec2) -> Panel {
    let rect = rows::dock(viewport.x);
    let nodes = model.map_or_else(Vec::new, |model| {
        let fill = (rect.width - DOCK_PAD * 2.0).max(0.0);
        let row = controls::row(ctx, &model.controls, RowKind::Dock, Some(fill));
        // Whole pixels, so the glyphs stay sharp.
        let corner = Vec2::new(
            rect.x + DOCK_PAD,
            (rect.y + (rect.height - row.height) / 2.0).round(),
        );
        (row.nodes.into_iter())
            .map(|node| node.moved(corner))
            .collect()
    });
    Panel {
        surface: Surface::Dock,
        rect,
        menu: false,
        nodes,
    }
}
