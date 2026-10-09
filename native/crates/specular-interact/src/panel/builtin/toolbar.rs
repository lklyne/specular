//! The toolbar: the second row of the chrome, with the tool buttons and the
//! zoom readout centred in it.

use glam::Vec2;
use specular_doc::TextAlign;

use super::super::{
    Dropdown, Icon, PaintRole, Palette, ThemeButton, ToolButton, ToolbarModel, ToolbarSection,
};
use super::controls::{self, RowKind, text};
use super::metrics::{
    CONTROL_RADIUS, DIVIDER, DIVIDER_MARGIN, DOCK_PAD, GAP, TOOL_BUTTON, TOOL_GLYPH, ZOOM_CHEVRON,
    ZOOM_PAD, ZOOM_TRIGGER,
};
use super::node::{Chrome, Node, Panel, PanelRect, Part, Run, Tint, Tone};
use super::{Ctx, rows};

/// The surface a tool glyph's color is resolved for: the pens show their
/// ink, the sticky and the shape their fill.
const fn tint_style(icon: Icon) -> (Palette, PaintRole) {
    match icon {
        Icon::DrawPenTool | Icon::DrawHighlightTool => (Palette::Vivid, PaintRole::Ink),
        _ => (Palette::Soft, PaintRole::Fill),
    }
}

fn tool(ctx: &Ctx<'_>, button: &ToolButton, left: f32) -> Node {
    let rect = PanelRect::new(left, 0.0, TOOL_BUTTON.0, TOOL_BUTTON.1);
    let (palette, role) = tint_style(button.icon);
    Node {
        id: Some(button.id.clone()),
        rect,
        radius: CONTROL_RADIUS,
        chrome: Chrome::ToolButton,
        state: ctx.state(&button.id, true, button.active),
        parts: vec![Part::Icon {
            icon: button.icon,
            rect: rect.centred(Vec2::splat(TOOL_GLYPH)),
            tint: button.tint.clone().map(|color| Tint {
                color,
                palette,
                role,
            }),
        }],
        run: Some(Run::Act {
            action: button.action.clone(),
            closes: true,
        }),
    }
}

fn theme(ctx: &Ctx<'_>, button: &ThemeButton, left: f32) -> Node {
    let rect = PanelRect::new(left, 0.0, TOOL_BUTTON.0, TOOL_BUTTON.1);
    Node {
        id: Some(button.id.clone()),
        rect,
        radius: CONTROL_RADIUS,
        chrome: Chrome::ToolButton,
        state: ctx.state(&button.id, true, false),
        parts: vec![Part::Icon {
            icon: button.icon,
            rect: rect.centred(Vec2::splat(TOOL_GLYPH)),
            tint: None,
        }],
        run: Some(Run::Act {
            action: button.action.clone(),
            closes: true,
        }),
    }
}

fn zoom(ctx: &Ctx<'_>, dropdown: &Dropdown, left: f32) -> Node {
    let rect = PanelRect::new(left, 0.0, ZOOM_TRIGGER.0, ZOOM_TRIGGER.1);
    let chevron = PanelRect::new(
        rect.right() - ZOOM_PAD.1 - ZOOM_CHEVRON,
        (rect.height - ZOOM_CHEVRON) / 2.0,
        ZOOM_CHEVRON,
        ZOOM_CHEVRON,
    );
    let words = PanelRect::new(
        left + ZOOM_PAD.0,
        0.0,
        chevron.x - left - ZOOM_PAD.0,
        rect.height,
    );
    let label = dropdown.summary.text.clone().unwrap_or_default();
    Node {
        id: Some(dropdown.id.clone()),
        rect,
        radius: CONTROL_RADIUS,
        chrome: Chrome::ToolMenu,
        state: ctx.state(
            &dropdown.id,
            true,
            ctx.ui.open.as_ref() == Some(&dropdown.id),
        ),
        parts: vec![
            text(label, words, TextAlign::Left, None, Tone::Follow),
            Part::Chevron { rect: chevron },
        ],
        run: Some(Run::Toggle),
    }
}

/// The toolbar of `model` across a viewport `viewport` wide.
pub(super) fn layout(ctx: &Ctx<'_>, model: &ToolbarModel, viewport: Vec2) -> Panel {
    let mut nodes = Vec::new();
    let mut left = 0.0;
    for (index, section) in model.sections.iter().enumerate() {
        if index > 0 {
            let line = PanelRect::new(
                left + DIVIDER_MARGIN,
                (TOOL_BUTTON.1 - DIVIDER.1) / 2.0,
                DIVIDER.0,
                DIVIDER.1,
            );
            nodes.push(Node::fixed(line, Chrome::Divider));
            left += DIVIDER.0 + DIVIDER_MARGIN * 2.0 + GAP;
        }
        match section {
            ToolbarSection::Tools(tools) => {
                for button in tools {
                    nodes.push(tool(ctx, button, left));
                    left += TOOL_BUTTON.0 + GAP;
                }
            }
            ToolbarSection::Zoom(dropdown) => {
                nodes.push(theme(ctx, &model.theme, left));
                left += TOOL_BUTTON.0 + GAP;
                nodes.push(zoom(ctx, dropdown, left));
                left += ZOOM_TRIGGER.0 + GAP;
            }
        }
    }
    let width = (left - GAP).max(0.0);
    let rect = rows::tools(viewport.x);
    let corner = Vec2::new(
        ((rect.width - width) / 2.0).round(),
        rect.y + (rect.height - TOOL_BUTTON.1) / 2.0,
    );
    let mut nodes: Vec<Node> = nodes.into_iter().map(|node| node.moved(corner)).collect();
    // The lens and the eye, from the row's right end.
    let view = controls::row(ctx, &model.view, RowKind::Dock, None);
    let width = controls::natural_width(ctx, &model.view, RowKind::Dock);
    let end = Vec2::new(
        (rect.width - DOCK_PAD - width).round(),
        (rect.y + (rect.height - view.height) / 2.0).round(),
    );
    nodes.extend(view.nodes.into_iter().map(|node| node.moved(end)));
    Panel {
        surface: super::Surface::Toolbar,
        rect,
        menu: false,
        nodes,
    }
}
