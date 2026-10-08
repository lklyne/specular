//! The menu a right-click on a canvas opens: Rename canvas, Delete canvas.

use glam::Vec2;
use specular_doc::{TextAlign, TextFont};

use super::super::metrics::EDGE_MARGIN;
use super::super::node::{Chrome, Node, Panel, PanelRect, Part, Run, Surface, Tone};
use super::super::{ContextMenu, Ctx};
use super::metrics::{MENU_INSET, MENU_ITEM, MENU_PAD, MENU_RADIUS, MENU_WIDTH};
use crate::{CanvasRow, SidebarModel};

/// The menu of `open`, or `None` once its canvas is gone.
pub(in super::super) fn layout(
    ctx: &Ctx<'_>,
    model: &SidebarModel,
    open: &ContextMenu,
    viewport: Vec2,
) -> Option<Panel> {
    let row: &CanvasRow = model.canvases.iter().find(|row| row.id == open.canvas)?;
    let size = Vec2::new(MENU_WIDTH, MENU_ITEM * 2.0 + MENU_INSET * 2.0);
    let corner = Vec2::new(
        open.at
            .x
            .min(viewport.x - EDGE_MARGIN - size.x)
            .max(EDGE_MARGIN),
        open.at
            .y
            .min(viewport.y - EDGE_MARGIN - size.y)
            .max(EDGE_MARGIN),
    )
    .round();
    let rect = PanelRect::new(corner.x, corner.y, size.x, size.y);
    let items = [
        ("rename", "Rename canvas", Run::Edit(row.rename.id.clone())),
        (
            "delete",
            "Delete canvas",
            Run::Act {
                action: row.delete.clone(),
                closes: true,
            },
        ),
    ];
    let nodes = items
        .into_iter()
        .enumerate()
        .map(|(index, (name, label, run))| {
            let id = row.control.child("menu").child(name);
            let item = PanelRect::new(
                rect.x + MENU_INSET,
                rect.y + MENU_INSET + MENU_ITEM * index as f32,
                MENU_WIDTH - MENU_INSET * 2.0,
                MENU_ITEM,
            );
            let words = PanelRect::new(
                item.x + MENU_PAD,
                item.y,
                item.width - MENU_PAD * 2.0,
                MENU_ITEM,
            );
            Node {
                radius: MENU_RADIUS,
                state: ctx.state(&id, true, false),
                parts: vec![Part::Text {
                    text: label.into(),
                    rect: words,
                    align: TextAlign::Left,
                    font: TextFont::Sans,
                    weight: 400,
                    tone: Tone::Strong,
                }],
                run: Some(run),
                id: Some(id),
                ..Node::fixed(item, Chrome::MenuRow)
            }
        })
        .collect();
    Some(Panel {
        surface: Surface::Dropdown,
        rect,
        menu: true,
        nodes,
    })
}
