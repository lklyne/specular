//! The nodes of the sidebar's heads and rows.

use specular_doc::{TextAlign, TextFont};

use super::super::Ctx;
use super::super::node::{Chrome, Input, Node, PanelRect, Part, Run, Tone};
use super::metrics::{
    ADD, ADD_GAP, ADD_RADIUS, CONTENT, EDGE, FOLD, FOLD_LEFT, GAP, HEAD, HEAD_GAP, HEAD_PAD,
    HEAD_WEIGHT, ICON, INPUT_HEIGHT, INPUT_LEFT, INPUT_PAD, INPUT_RADIUS, INPUT_TOP, PAD_LEFT,
    PAD_RIGHT, ROW, STEP,
};
use super::{Entry, ellipsize};
use crate::panel::{ControlId, Icon};
use crate::{CanvasRow, SectionHead, SidebarModel, SidebarRow};

/// The field being edited in place, if one is.
pub(super) fn editing(ctx: &Ctx<'_>) -> Option<ControlId> {
    let edit = ctx.app.text_edit().filter(|edit| edit.is_field())?;
    Some(ControlId::from(edit.entity().as_str().to_owned()))
}

/// `rect` cut to `window`, so a row half out of the list takes presses only
/// where it shows.
fn within(rect: PanelRect, window: PanelRect) -> PanelRect {
    let left = rect.x.max(window.x);
    let top = rect.y.max(window.y);
    let right = rect.right().min(window.right());
    let bottom = rect.bottom().min(window.bottom());
    PanelRect::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0))
}

/// What pressing a node runs: `action`.
fn act(action: &crate::Action) -> Run {
    Run::Act {
        action: action.clone(),
        closes: true,
    }
}

fn glyph(icon: Icon, rect: PanelRect, tone: Tone) -> Part {
    Part::Glyph { icon, rect, tone }
}

fn text(text: String, rect: PanelRect, align: TextAlign, weight: u16, tone: Tone) -> Part {
    Part::Text {
        text: text.into(),
        rect,
        align,
        font: TextFont::Sans,
        weight,
        tone,
    }
}

/// A head's title and the chevron after it, in a button `width` wide whose
/// top-left corner is `at`.
fn head(ctx: &Ctx<'_>, head: &SectionHead, at: (f32, f32), width: f32) -> Node {
    let rect = PanelRect::new(at.0, at.1, width, HEAD);
    let room = width - HEAD_GAP - FOLD;
    let title = ellipsize(ctx, &head.title, room);
    let title_width = ctx.text_width(&title, TextFont::Sans);
    let chevron = PanelRect::new(
        rect.x + title_width + HEAD_GAP,
        rect.y + (HEAD - FOLD) / 2.0,
        FOLD,
        FOLD,
    );
    let icon = if head.folded {
        Icon::ChevronRight
    } else {
        Icon::ChevronDown
    };
    Node {
        id: Some(head.id.clone()),
        state: ctx.state(&head.id, true, false),
        parts: vec![
            text(
                title,
                PanelRect::new(rect.x, rect.y, title_width, HEAD),
                TextAlign::Left,
                HEAD_WEIGHT,
                Tone::Strong,
            ),
            glyph(icon, chevron, Tone::Strong),
        ],
        run: Some(act(&head.toggle)),
        ..Node::fixed(rect, Chrome::Plain)
    }
}

/// The Canvases head with the add button, across the top of the sidebar.
pub(super) fn frame_head(
    ctx: &Ctx<'_>,
    model: &SidebarModel,
    frame: PanelRect,
    out: &mut Vec<Node>,
) {
    let inner = CONTENT - HEAD_PAD * 2.0;
    let button = inner - ADD_GAP - ADD;
    out.push(head(ctx, &model.canvases_head, (HEAD_PAD, frame.y), button));
    let add_id = ControlId::new("sidebar.add");
    let add = PanelRect::new(
        HEAD_PAD + button + ADD_GAP,
        frame.y + (HEAD - ADD) / 2.0,
        ADD,
        ADD,
    );
    out.push(Node {
        id: Some(add_id.clone()),
        radius: ADD_RADIUS,
        state: ctx.state(&add_id, true, false),
        parts: vec![glyph(
            Icon::Plus,
            add.centred(glam::Vec2::splat(ICON)),
            Tone::Follow,
        )],
        run: Some(act(&model.add_canvas)),
        ..Node::fixed(add, Chrome::Subtle)
    });
    if model.canvases_head.folded {
        let line = PanelRect::new(0.0, frame.y + HEAD - EDGE, CONTENT, EDGE);
        out.push(Node::fixed(line, Chrome::Edge));
    }
}

/// The nodes of one `entry` whose band is `band`, seen through `window`.
pub(super) fn entry(
    ctx: &Ctx<'_>,
    entry: &Entry<'_>,
    band: PanelRect,
    window: PanelRect,
    editing: Option<&ControlId>,
    out: &mut Vec<Node>,
) {
    match entry {
        Entry::Gap(_) => {}
        Entry::Rule => out.push(Node::fixed(band, Chrome::Edge)),
        Entry::Head(section) => {
            let node = head(
                ctx,
                section,
                (HEAD_PAD, band.y),
                band.width - HEAD_PAD * 2.0,
            );
            out.push(Node {
                rect: within(node.rect, window),
                ..node
            });
        }
        Entry::Canvas(row) => canvas(ctx, row, band, window, editing, out),
        Entry::Row(row, depth) => entity(ctx, row, *depth, band, window, out),
        Entry::Empty => {
            let area = PanelRect::new(
                PAD_LEFT,
                band.y + 4.0,
                band.width - PAD_LEFT - PAD_RIGHT,
                14.0,
            );
            out.push(Node {
                parts: vec![text(
                    "No items".to_owned(),
                    area,
                    TextAlign::Left,
                    400,
                    Tone::Muted,
                )],
                ..Node::fixed(area, Chrome::Plain)
            });
        }
    }
}

fn canvas(
    ctx: &Ctx<'_>,
    row: &CanvasRow,
    band: PanelRect,
    window: PanelRect,
    editing: Option<&ControlId>,
    out: &mut Vec<Node>,
) {
    let label_x = PAD_LEFT + ICON + GAP;
    let check = PanelRect::new(
        band.width - PAD_RIGHT - ICON,
        band.y + (ROW - ICON) / 2.0,
        ICON,
        ICON,
    );
    let end = if row.active {
        check.x - GAP
    } else {
        band.width - PAD_RIGHT
    };
    let mut parts = vec![glyph(
        Icon::File,
        PanelRect::new(PAD_LEFT, band.y + (ROW - ICON) / 2.0, ICON, ICON),
        Tone::Muted,
    )];
    let renaming = editing == Some(&row.rename.id);
    if !renaming {
        let label = ellipsize(ctx, &row.label, end - label_x);
        let width = ctx.text_width(&label, TextFont::Sans);
        let area = PanelRect::new(label_x, band.y, width, ROW);
        parts.push(text(label, area, TextAlign::Left, 400, Tone::Strong));
    }
    if row.active {
        parts.push(glyph(Icon::Check, check, Tone::Strong));
    }
    out.push(Node {
        id: Some(row.control.clone()),
        state: ctx.state(&row.control, true, false),
        parts,
        run: Some(act(&row.action)),
        ..Node::fixed(within(band, window), Chrome::Row)
    });
    if renaming {
        out.push(rename_input(row, label_x, end, band.y, window));
    }
}

/// The box a canvas's name is edited in, where its label was.
fn rename_input(row: &CanvasRow, label_x: f32, end: f32, top: f32, window: PanelRect) -> Node {
    let rect = PanelRect::new(
        label_x - INPUT_LEFT,
        top + INPUT_TOP,
        end + 1.0 - (label_x - INPUT_LEFT),
        INPUT_HEIGHT,
    );
    let area = PanelRect::new(
        label_x,
        top + (ROW - 16.0) / 2.0,
        (end - INPUT_PAD - label_x).max(0.0),
        16.0,
    );
    Node {
        id: Some(row.rename.id.clone()),
        radius: INPUT_RADIUS,
        parts: vec![Part::Input(Input {
            text: row.rename.value.clone().into(),
            hint: None,
            area,
            scroll: 0.0,
            focus: None,
        })],
        ..Node::fixed(within(rect, window), Chrome::InlineInput)
    }
}

fn entity(
    ctx: &Ctx<'_>,
    row: &SidebarRow,
    depth: usize,
    band: PanelRect,
    window: PanelRect,
    out: &mut Vec<Node>,
) {
    let pad = PAD_LEFT + STEP * depth as f32;
    let mut parts = vec![glyph(
        row.glyph,
        PanelRect::new(pad, band.y + (ROW - ICON) / 2.0, ICON, ICON),
        Tone::Muted,
    )];
    let mut end = band.width - PAD_RIGHT;
    if let Some(trailing) = &row.trailing {
        let width = ctx.text_width(trailing, TextFont::Sans);
        let area = PanelRect::new(end - width, band.y, width, ROW);
        parts.push(text(
            trailing.clone(),
            area,
            TextAlign::Right,
            400,
            Tone::Muted,
        ));
        end -= width + GAP;
    }
    let label_x = pad + ICON + GAP;
    let label = ellipsize(ctx, &row.label, end - label_x);
    let width = ctx.text_width(&label, TextFont::Sans);
    parts.push(text(
        label,
        PanelRect::new(label_x, band.y, width, ROW),
        TextAlign::Left,
        400,
        Tone::Strong,
    ));
    let mut state = ctx.state(&row.id, true, row.selected);
    state.dimmed = row.dimmed;
    out.push(Node {
        id: Some(row.id.clone()),
        state,
        parts,
        run: Some(act(&row.action)),
        ..Node::fixed(within(band, window), Chrome::Row)
    });
    if let (Some(toggle), Some(open)) = (&row.toggle, row.expanded) {
        let id = row.id.child("toggle");
        let icon = if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        let hit = PanelRect::new(pad - FOLD_LEFT - 2.0, band.y, FOLD + 4.0, ROW);
        let chevron = PanelRect::new(pad - FOLD_LEFT, band.y + (ROW - FOLD) / 2.0, FOLD, FOLD);
        out.push(Node {
            state: ctx.state(&id, true, false),
            parts: vec![glyph(icon, chevron, Tone::Muted)],
            run: Some(act(toggle)),
            id: Some(id),
            ..Node::fixed(within(hit, window), Chrome::Plain)
        });
    }
}
