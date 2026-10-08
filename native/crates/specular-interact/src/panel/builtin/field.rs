//! A text field laid out: its box, its caption, and while it is edited the
//! caret and selection the editor holds for it.

use std::sync::Arc;

use specular_doc::TextAlign;

use super::super::{ControlId, Field, FieldWidth};
use super::controls::text;
use super::metrics::{
    CONTROL_RADIUS, FIELD_HEIGHT, FIELD_MEDIUM, FIELD_PAD, FIELD_SHORT, FIELD_WIDE, GAP, TEXT_LINE,
};
use super::node::{Chrome, Input, InputFocus, Node, PanelRect, Part, Tone};
use super::{Ctx, PanelLayout};
use crate::App;
use crate::geometry::ScreenRect;

/// The least width of a field's box.
fn box_width(field: &Field) -> f32 {
    match field.width {
        FieldWidth::Wide => FIELD_WIDE,
        FieldWidth::Short => FIELD_SHORT,
        FieldWidth::Medium => FIELD_MEDIUM,
    }
}

fn caption_width(ctx: &Ctx<'_>, field: &Field) -> f32 {
    (field.caption.as_ref()).map_or(0.0, |caption| {
        ctx.text_width(caption, specular_doc::TextFont::Sans) + GAP
    })
}

/// The width a field takes with nothing to spare: its caption and its box.
pub(super) fn natural_width(ctx: &Ctx<'_>, field: &Field) -> f32 {
    caption_width(ctx, field) + box_width(field)
}

/// Where the line of text sits in a field's box: inside the border and the
/// side padding.
pub(crate) fn text_area(rect: PanelRect) -> PanelRect {
    PanelRect::new(
        rect.x + 1.0 + FIELD_PAD,
        rect.y + 1.0,
        (rect.width - 2.0 - FIELD_PAD * 2.0).max(0.0),
        (rect.height - 2.0).max(0.0),
    )
}

/// The field `field`, `left` along its row and `width` wide with its
/// caption, as nodes.
pub(super) fn nodes(ctx: &Ctx<'_>, field: &Field, left: f32, width: f32, out: &mut Vec<Node>) {
    let lead = caption_width(ctx, field);
    if let Some(caption) = &field.caption {
        let area = PanelRect::new(left, 0.0, lead - GAP, FIELD_HEIGHT);
        out.push(Node {
            parts: vec![text(
                caption.clone(),
                area,
                TextAlign::Left,
                None,
                Tone::Muted,
            )],
            ..Node::fixed(area, Chrome::Plain)
        });
    }
    let rect = PanelRect::new(left + lead, 0.0, width - lead, FIELD_HEIGHT);
    out.push(Node {
        id: Some(field.id.clone()),
        radius: CONTROL_RADIUS,
        state: ctx.state(&field.id, true, false),
        parts: vec![Part::Input(Input {
            text: field.value.clone().into(),
            hint: field.placeholder.clone(),
            area: text_area(rect),
            scroll: 0.0,
            focus: None,
        })],
        ..Node::fixed(rect, Chrome::Input)
    });
}

/// Whether `chrome` is the box of a text field.
pub(super) const fn is_field(chrome: Chrome) -> bool {
    matches!(chrome, Chrome::Input | Chrome::InlineInput)
}

/// The box of the field named `id`, as the panels lay it out now, or `None`
/// when no panel shows it.
pub(crate) fn field_box(app: &App, id: &ControlId) -> Option<PanelRect> {
    let layout = super::cache::base(app);
    let node = layout.node(id)?;
    is_field(node.chrome).then_some(node.rect)
}

/// The box the line of the field named `id` is laid out in and clipped to.
pub(crate) fn field_text_area(app: &App, id: &ControlId) -> Option<PanelRect> {
    let layout = super::cache::base(app);
    let node = layout.node(id)?;
    node.parts.iter().find_map(|part| match part {
        Part::Input(input) => Some(input.area),
        Part::Icon { .. }
        | Part::Text { .. }
        | Part::Dot { .. }
        | Part::Chevron { .. }
        | Part::Check { .. }
        | Part::Key { .. }
        | Part::Glyph { .. } => None,
    })
}

fn on_screen(app: &App, rect: specular_doc::Rect) -> PanelRect {
    let shown = ScreenRect::of(&app.session.camera, rect);
    PanelRect::new(shown.min.x, shown.min.y, shown.size.x, shown.size.y)
}

/// `base` with what the editor holds for the field being edited put over it.
/// Shares `base` when no field is edited.
pub(super) fn overlay(app: &App, base: Arc<PanelLayout>) -> Arc<PanelLayout> {
    if !app.text_edit().is_some_and(crate::edit::TextEdit::is_field) {
        return base;
    }
    let mut edited = (*base).clone();
    overlay_in_place(app, &mut edited);
    Arc::new(edited)
}

/// Puts what the editor holds for the field being edited into its node: the
/// text typed so far, scrolled, with the caret and selection over it.
pub(super) fn overlay_in_place(app: &App, layout: &mut PanelLayout) {
    let Some(edit) = app.text_edit().filter(|edit| edit.is_field()) else {
        return;
    };
    let id = ControlId::from(edit.entity().as_str().to_owned());
    let panels = [
        &mut layout.sidebar_list,
        &mut layout.popup,
        &mut layout.dropdown,
    ];
    let node = panels
        .into_iter()
        .flatten()
        .flat_map(|panel| panel.nodes.iter_mut())
        .find(|node| node.id.as_ref() == Some(&id));
    let Some(node) = node else {
        return;
    };
    node.state.on = true;
    for part in &mut node.parts {
        let Part::Input(input) = part else {
            continue;
        };
        let rects = |rects: Vec<specular_doc::Rect>| -> Vec<PanelRect> {
            rects.into_iter().map(|rect| on_screen(app, rect)).collect()
        };
        let selection = rects(app.selection_rects());
        let caret = (selection.is_empty() && app.caret_visible())
            .then(|| app.caret_rect())
            .flatten()
            .map(|rect| {
                let line = on_screen(app, rect);
                PanelRect::new(line.x.round(), line.y, 1.0, line.height.min(TEXT_LINE))
            });
        input.text = edit.text().to_owned().into();
        input.scroll = edit.scroll;
        input.focus = Some(InputFocus {
            selection,
            composition: rects(app.composition_rects()),
            caret,
        });
    }
}
