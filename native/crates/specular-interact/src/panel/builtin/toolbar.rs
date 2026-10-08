//! The toolbar: a strip across the top of the viewport with the tool
//! buttons and the zoom readout centred in it.

use glam::Vec2;
use specular_doc::TextAlign;

use super::super::{
    Dropdown, Icon, PaintRole, Palette, SidebarButton, ThemeButton, ToolButton, ToolbarModel,
    ToolbarSection,
};
use super::Ctx;
use super::controls::text;
use super::metrics::{
    CONTROL_RADIUS, DIVIDER, DIVIDER_MARGIN, GAP, ICON, SIDEBAR_BUTTON, SIDEBAR_BUTTON_LEFT,
    SIDEBAR_BUTTON_RADIUS, TOOL_BUTTON, TOOL_GLYPH, TOOLBAR_HEIGHT, ZOOM_CHEVRON, ZOOM_PAD,
    ZOOM_TRIGGER,
};
use super::node::{Chrome, Node, Panel, PanelRect, Part, Run, Tint, Tone};

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
    let corner = Vec2::new(
        ((viewport.x - width) / 2.0).round(),
        (TOOLBAR_HEIGHT - TOOL_BUTTON.1) / 2.0,
    );
    let mut nodes: Vec<Node> = nodes.into_iter().map(|node| node.moved(corner)).collect();
    nodes.push(sidebar_button(ctx, &model.sidebar));
    Panel {
        surface: super::Surface::Toolbar,
        rect: PanelRect::new(0.0, 0.0, viewport.x, TOOLBAR_HEIGHT),
        menu: false,
        nodes,
    }
}

/// The sidebar's button at the strip's left edge: `p-1.5` around a 14 px
/// glyph, set in from the edge by the toolbar's `px-4`, and faded while the
/// sidebar is hidden.
fn sidebar_button(ctx: &Ctx<'_>, button: &SidebarButton) -> Node {
    let rect = PanelRect::new(
        SIDEBAR_BUTTON_LEFT,
        (TOOLBAR_HEIGHT - SIDEBAR_BUTTON) / 2.0,
        SIDEBAR_BUTTON,
        SIDEBAR_BUTTON,
    );
    let mut state = ctx.state(&button.id, true, false);
    state.dimmed = !button.open;
    Node {
        id: Some(button.id.clone()),
        radius: SIDEBAR_BUTTON_RADIUS,
        state,
        parts: vec![Part::Glyph {
            icon: button.icon,
            rect: rect.centred(Vec2::splat(ICON)),
            tone: Tone::Follow,
        }],
        run: Some(Run::Act {
            action: button.action.clone(),
            closes: true,
        }),
        ..Node::fixed(rect, Chrome::Subtle)
    }
}
