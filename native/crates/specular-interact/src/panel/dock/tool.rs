//! The dock's options for the tool in hand (`TextToolPopup.tsx`,
//! `ShapeToolPopup.tsx`, `DrawToolPopup.tsx`). Their controls write the
//! tool defaults.

use specular_doc::{Color, VIEWPORT_PRESETS};

use super::super::build::{font_dropdown, groups, size_dropdown, swatches};
use super::super::{Control, ControlId, ControlsModel, Dropdown, Face, PaintRole, Palette};
use super::{drawing, shape};
use crate::{Action, App, Tool, ToolDefaultPatch};

fn set(patch: ToolDefaultPatch) -> Action {
    Action::SetToolDefault(patch)
}

fn docked(controls: Vec<Control>) -> ControlsModel {
    ControlsModel { controls }
}

/// The preset the next page is made at.
fn page_presets(app: &App) -> ControlsModel {
    let page = app.tool_defaults().page;
    let id = ControlId::new("page.preset");
    let named = (!page.custom)
        .then(|| VIEWPORT_PRESETS.get(usize::try_from(page.preset).ok()?))
        .flatten();
    docked(vec![Control::Dropdown(Dropdown {
        label: "Page size to add".into(),
        summary: Face::text(named.map_or("Custom", |row| row.label)),
        content: super::page::preset_sections(
            &id,
            (!page.custom).then_some(page.preset),
            Some("Add"),
            |index| set(ToolDefaultPatch::PagePreset(index)),
            Some((page.custom, set(ToolDefaultPatch::PageCustom))),
        ),
        id,
    })])
}

pub(super) fn controls(app: &App, tool: Tool) -> Option<ControlsModel> {
    let defaults = app.tool_defaults();
    let model = match tool {
        Tool::AddText => {
            let ink = defaults.text.color.clone().unwrap_or(Color::Neutral);
            docked(groups(vec![
                vec![
                    size_dropdown(
                        ControlId::new("text.size"),
                        "Set default text size",
                        Some(defaults.text.size),
                        |size| set(ToolDefaultPatch::TextSize(size)),
                    ),
                    font_dropdown(
                        ControlId::new("text.font"),
                        "Set default text font",
                        Some(defaults.text.font),
                        |font| set(ToolDefaultPatch::TextFont(font)),
                    ),
                ],
                vec![Control::Swatches(swatches(
                    ControlId::new("text.color"),
                    Palette::Vivid,
                    PaintRole::Ink,
                    Some(&ink),
                    None,
                    |color| set(ToolDefaultPatch::TextColor(Some(color))),
                ))],
            ]))
        }
        Tool::AddSticky => docked(groups(vec![
            vec![
                size_dropdown(
                    ControlId::new("sticky.size"),
                    "Set default sticky text size",
                    Some(defaults.sticky.size),
                    |size| set(ToolDefaultPatch::StickySize(size)),
                ),
                font_dropdown(
                    ControlId::new("sticky.font"),
                    "Set default sticky text font",
                    Some(defaults.sticky.font),
                    |font| set(ToolDefaultPatch::StickyFont(font)),
                ),
            ],
            vec![Control::Swatches(swatches(
                ControlId::new("sticky.color"),
                Palette::Soft,
                PaintRole::Fill,
                Some(&defaults.sticky.color),
                None,
                |color| set(ToolDefaultPatch::StickyColor(color)),
            ))],
        ])),
        Tool::AddShape => docked(groups(vec![
            vec![shape::kind_dropdown(
                ControlId::new("shape.kind"),
                "Set default shape",
                Some(defaults.shape.kind),
                |kind| set(ToolDefaultPatch::ShapeKind(kind)),
            )],
            vec![size_dropdown(
                ControlId::new("shape.size"),
                "Set default label size",
                Some(defaults.shape.text_size),
                |size| set(ToolDefaultPatch::ShapeTextSize(size)),
            )],
            vec![Control::Swatches(swatches(
                ControlId::new("shape.color"),
                Palette::Soft,
                PaintRole::Fill,
                Some(&defaults.shape.color),
                None,
                |color| set(ToolDefaultPatch::ShapeColor(color)),
            ))],
        ])),
        Tool::Draw => {
            let draw = &defaults.draw;
            docked(groups(vec![
                drawing::brushes(Some(&draw.color), Some(draw.brush), |brush| {
                    set(ToolDefaultPatch::Brush(brush))
                }),
                drawing::widths(draw.brush, Some(draw.stroke_width), |width| {
                    set(ToolDefaultPatch::DrawStrokeWidth(width))
                }),
                vec![Control::Swatches(swatches(
                    ControlId::new("draw.color"),
                    drawing::palette_of(draw.brush),
                    PaintRole::Ink,
                    Some(&draw.color),
                    None,
                    |color| set(ToolDefaultPatch::DrawColor(color)),
                ))],
            ]))
        }
        Tool::AddPage => page_presets(app),
        Tool::Select | Tool::AddDocument | Tool::Comment | Tool::Inspect => return None,
    };
    Some(model)
}
