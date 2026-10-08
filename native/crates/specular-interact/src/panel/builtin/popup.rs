//! The popup: one row of controls in a floating frame.

use glam::Vec2;

use super::super::{Align, PopupAnchor, PopupModel};
use super::controls::{self, RowKind};
use super::metrics::{CONTROL, EDGE_MARGIN, INSET, TOOLBAR_HEIGHT};
use super::node::{Panel, PanelRect, Surface};
use super::{Ctx, place};
use crate::geometry::ScreenRect;

/// The popup of `model`, or `None` when what it points at is off screen.
pub(super) fn layout(ctx: &Ctx<'_>, model: &PopupModel, viewport: Vec2) -> Option<Panel> {
    let row = controls::row(ctx, &model.controls, RowKind::Popup, None);
    let mut size = Vec2::new(row.width, CONTROL) + Vec2::splat(INSET * 2.0);
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
    // Whole pixels, so the frame's hairline and its glyphs stay sharp.
    let corner = corner.round();
    let content = corner + Vec2::splat(INSET);
    Some(Panel {
        surface: Surface::Popup,
        rect: PanelRect::new(corner.x, corner.y, size.x, size.y),
        menu: false,
        nodes: (row.nodes.into_iter())
            .map(|node| node.moved(content))
            .collect(),
    })
}
