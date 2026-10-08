//! The toolbar: a strip across the top of the viewport with the tool
//! buttons and the zoom readout centred in it.

use glam::Vec2;
use specular_doc::TextAlign;

use super::super::{Dropdown, Icon, PaintRole, Palette, ToolButton, ToolbarModel, ToolbarSection};
use super::Ctx;
use super::controls::text;
use super::metrics::{
    CONTROL_RADIUS, DIVIDER, DIVIDER_MARGIN, GAP, TOOL_BUTTON, TOOL_GLYPH, TOOLBAR_HEIGHT,
    ZOOM_CHEVRON, ZOOM_PAD, ZOOM_TRIGGER,
};
use super::node::{Chrome, Node, Panel, PanelRect, Part, Run, Tint, Tone};

/// The surface a tool glyph's color is resolved for: the pens show their
/// ink, the sticky and the shape their fill.
const fn tint_style(icon: Icon) -> (Palette, PaintRole) {
    match icon {
        Icon::DrawPenTool | Icon::DrawHighlightTool => (Palette::Vivid, PaintRole::Ink),
        Icon::SelectTool
        | Icon::PageTool
        | Icon::TextTool
        | Icon::StickyTool
        | Icon::DocumentTool
        | Icon::ShapeTool
        | Icon::CommentTool
        | Icon::Shape(_)
        | Icon::AlignLeft
        | Icon::AlignCenter
        | Icon::AlignRight
        | Icon::BrushPen
        | Icon::BrushHighlighter
        | Icon::StrokeThin
        | Icon::StrokeThick
        | Icon::Border
        | Icon::LineSolid
        | Icon::LineDashed
        | Icon::Ban
        | Icon::ArrowStart
        | Icon::ArrowEnd
        | Icon::Trash
        | Icon::Bold
        | Icon::Strikethrough
        | Icon::BulletList
        | Icon::Device
        | Icon::Rotate
        | Icon::SchemeSystem
        | Icon::SchemeLight
        | Icon::SchemeDark => (Palette::Soft, PaintRole::Fill),
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
    Panel {
        surface: super::Surface::Toolbar,
        rect: PanelRect::new(0.0, 0.0, viewport.x, TOOLBAR_HEIGHT),
        menu: false,
        nodes: nodes.into_iter().map(|node| node.moved(corner)).collect(),
    }
}
