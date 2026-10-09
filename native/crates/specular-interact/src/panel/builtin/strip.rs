//! The tab row: the first row of the chrome, with a tab for the canvas and
//! one for each page and Document.

use glam::Vec2;
use specular_doc::TextAlign;

use super::super::{Button, Icon, ViewStrip, ViewTab};
use super::controls::text;
use super::metrics::{
    CONTROL_RADIUS, GAP, ICON, TAB, TAB_BARE, TAB_LABEL_MIN, TAB_PAD, TAB_ROW, TABS_LEFT,
};
use super::node::{Chrome, Node, Panel, PanelRect, Part, Run, Surface, Tone};
use super::sidebar::ellipsize;
use super::{Ctx, rows};

/// How wide each of `count` tabs is in a row `width` wide, and how many of
/// them fit. They share what the traffic lights, the add button and the bare
/// end of the row leave, none wider than a tab at rest. Once they are down
/// to their glyphs the ones that do not fit are left out, so the bare end is
/// always there to drag the window by.
fn share(count: usize, width: f32) -> (f32, usize) {
    let room = (width - TABS_LEFT - TAB_BARE - TAB.2 - GAP).max(0.0);
    let even = (room + GAP) / count.max(1) as f32 - GAP;
    let each = even.clamp(TAB.1, TAB.0).floor();
    let fit = ((room + GAP) / (each + GAP)).floor() as usize;
    (each, fit.min(count))
}

fn tab(ctx: &Ctx<'_>, model: &ViewTab, left: f32, width: f32) -> Node {
    let rect = PanelRect::new(left, (TAB_ROW - TAB.2) / 2.0, width, TAB.2);
    let glyph = PanelRect::new(left + TAB_PAD, rect.y + (TAB.2 - ICON) / 2.0, ICON, ICON);
    let words_left = glyph.right() + GAP;
    let room = rect.right() - TAB_PAD - words_left;
    let mut parts = vec![Part::Icon {
        icon: model.icon,
        rect: glyph,
        tint: None,
    }];
    if room >= TAB_LABEL_MIN {
        let words = PanelRect::new(words_left, rect.y, room, rect.height);
        let label = ellipsize(ctx, &model.label, room);
        parts.push(text(label, words, TextAlign::Left, None, Tone::Follow));
    }
    Node {
        id: Some(model.id.clone()),
        radius: CONTROL_RADIUS,
        state: ctx.state(&model.id, true, model.active),
        parts,
        run: Some(Run::Act {
            action: model.action.clone(),
            closes: true,
        }),
        ..Node::fixed(rect, Chrome::ToolButton)
    }
}

/// The add button, a square as tall as a tab, after the last tab drawn.
fn add(ctx: &Ctx<'_>, model: &Button, left: f32) -> Node {
    let rect = PanelRect::new(left, (TAB_ROW - TAB.2) / 2.0, TAB.2, TAB.2);
    Node {
        id: Some(model.id.clone()),
        radius: CONTROL_RADIUS,
        state: ctx.state(&model.id, model.enabled, false),
        parts: vec![Part::Icon {
            icon: model.face.icon.unwrap_or(Icon::Plus),
            rect: rect.centred(Vec2::splat(ICON)),
            tint: None,
        }],
        run: Some(Run::Act {
            action: model.action.clone(),
            closes: true,
        }),
        ..Node::fixed(rect, Chrome::ToolButton)
    }
}

/// The tab row of `model` across a viewport `viewport` wide.
pub(super) fn layout(ctx: &Ctx<'_>, model: &ViewStrip, viewport: Vec2) -> Panel {
    let (each, fit) = share(model.tabs.len(), viewport.x);
    let mut nodes: Vec<Node> = (model.tabs.iter().take(fit).enumerate())
        .map(|(index, it)| tab(ctx, it, TABS_LEFT + index as f32 * (each + GAP), each))
        .collect();
    nodes.push(add(ctx, &model.add, TABS_LEFT + fit as f32 * (each + GAP)));
    Panel {
        surface: Surface::Tabs,
        rect: rows::tabs(viewport.x),
        menu: false,
        nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_share_the_row_and_leave_its_end_bare() {
        // 1280 wide leaves 1082 between the traffic lights and the add
        // button, which the bare end follows.
        let rows = [
            (3, (180.0, 3)),
            (10, (104.0, 10)),
            (30, (32.0, 30)),
            (40, (28.0, 33)),
        ];
        for (count, want) in rows {
            assert_eq!(share(count, 1280.0), want, "{count} tabs");
        }
    }
}
