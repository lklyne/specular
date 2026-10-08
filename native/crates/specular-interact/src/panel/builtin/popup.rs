//! The popup: one row of controls in a floating frame.

use glam::Vec2;

use super::super::{Align, Control, PopupAnchor, PopupModel};
use super::controls::{self, RowKind};
use super::dropdown;
use super::metrics::{EDGE_MARGIN, INSET, TOOLBAR_HEIGHT};
use super::node::{Node, Panel, PanelRect, Surface};
use super::{Ctx, place};
use crate::geometry::ScreenRect;

/// The controls of a popup laid out from the corner of their content: a
/// row, or the one list of choices that fills the popup.
fn content(ctx: &Ctx<'_>, controls: &[Control]) -> (Vec<Node>, Vec2) {
    if let [Control::Choices(choices)] = controls {
        let body = dropdown::body(ctx, &choices.content);
        return (body.nodes, body.size);
    }
    let row = controls::row(ctx, controls, RowKind::Popup, None);
    (row.nodes, Vec2::new(row.width, row.height))
}

/// The popup of `model`, or `None` when what it points at is off screen.
pub(super) fn layout(ctx: &Ctx<'_>, model: &PopupModel, viewport: Vec2) -> Option<Panel> {
    let (mut nodes, inner) = content(ctx, &model.controls);
    let mut size = inner + Vec2::splat(INSET * 2.0);
    let corner = match model.anchor {
        PopupAnchor::Toolbar { gap } => {
            Vec2::new((viewport.x - size.x) / 2.0, TOOLBAR_HEIGHT + gap)
        }
        PopupAnchor::Canvas {
            bounds,
            placement,
            align,
            gap,
        } => {
            let on_screen = ScreenRect::of(&ctx.app.session.camera, bounds);
            let anchor = PanelRect::new(
                on_screen.min.x,
                on_screen.min.y,
                on_screen.size.x,
                on_screen.size.y,
            );
            match align {
                Align::Center => {}
                // At least as wide as what it points at, as far as the
                // viewport lets it be.
                Align::Stretch => {
                    let most = (viewport.x - EDGE_MARGIN * 2.0).max(0.0);
                    size.x = size.x.max(anchor.width.min(most));
                }
            }
            place::beside(anchor, placement, gap, size, viewport)?
        }
    };
    // A popup wider than its content gives the room to what stretches.
    if size.x - INSET * 2.0 > inner.x + 0.5 {
        let fill = Some(size.x - INSET * 2.0);
        nodes = controls::row(ctx, &model.controls, RowKind::Popup, fill).nodes;
    }
    // Whole pixels, so the frame's hairline and its glyphs stay sharp.
    let corner = corner.round();
    let content = corner + Vec2::splat(INSET);
    Some(Panel {
        surface: Surface::Popup,
        rect: PanelRect::new(corner.x, corner.y, size.x, size.y),
        menu: false,
        nodes: (nodes.into_iter())
            .map(|node| node.moved(content))
            .collect(),
    })
}
