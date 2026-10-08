//! The sidebar laid out: a frame with the head of the Canvases list, and
//! under it a list that scrolls and is seen through a window.
//!
//! `left-sidebar/App.tsx` is the spec. The frame and the list are two
//! panels because the list's box is the clip its nodes are drawn through.

mod menu;
mod metrics;
mod rows;

use glam::Vec2;
use specular_core::Modifiers;
use specular_doc::{ItemId, TextFont};

pub(super) use self::menu::layout as context_menu;
use self::metrics::{CONTENT, HEAD, ROW, SCROLLBAR, THUMB_MIN};
use super::Ctx;
use super::metrics::TOOLBAR_HEIGHT;
use super::node::{Chrome, Node, Panel, PanelRect, Surface};
use crate::panel::ControlId;
use crate::{
    Action, App, CanvasRow, RowKind, RowTarget, SIDEBAR_WIDTH, SectionHead, SidebarModel,
    SidebarRow,
};

/// One thing in the scrolling list.
pub(crate) enum Entry<'a> {
    /// Room, in pixels.
    Gap(f32),
    /// A canvas.
    Canvas(&'a CanvasRow),
    /// The line between the canvases and the sections.
    Rule,
    /// The head of Notes or Pages.
    Head(&'a SectionHead),
    /// A row, `depth` levels in.
    Row(&'a SidebarRow, usize),
    /// The note that the canvas has nothing to list.
    Empty,
}

impl Entry<'_> {
    fn height(&self) -> f32 {
        match self {
            Self::Gap(height) => *height,
            Self::Canvas(_) | Self::Row(..) => ROW,
            Self::Rule => metrics::RULE,
            Self::Head(_) => HEAD,
            Self::Empty => metrics::EMPTY,
        }
    }
}

fn push_rows<'a>(out: &mut Vec<Entry<'a>>, rows: &'a [SidebarRow], depth: usize) {
    for row in rows {
        out.push(Entry::Row(row, depth));
        if row.expanded == Some(true) {
            push_rows(out, &row.children, depth + 1);
        }
    }
}

/// The list top to bottom: the canvases unless folded, then the sections.
pub(crate) fn entries(model: &SidebarModel) -> Vec<Entry<'_>> {
    let mut out = Vec::new();
    if !model.canvases_head.folded {
        out.push(Entry::Gap(metrics::LIST_TOP));
        out.extend(model.canvases.iter().map(Entry::Canvas));
        out.push(Entry::Gap(metrics::LIST_BOTTOM));
    }
    out.push(Entry::Rule);
    out.push(Entry::Gap(metrics::SECTIONS_PAD));
    for (head, list) in [
        (&model.notes_head, &model.notes),
        (&model.pages_head, &model.pages),
    ] {
        out.push(Entry::Head(head));
        if !head.folded {
            push_rows(&mut out, list, 0);
        }
    }
    if model.is_empty() {
        out.push(Entry::Empty);
    }
    out.push(Entry::Gap(metrics::SECTIONS_PAD));
    out
}

/// The sidebar's box under the toolbar, and the box of its list.
fn boxes(viewport: Vec2) -> (PanelRect, PanelRect) {
    let frame = PanelRect::new(
        0.0,
        TOOLBAR_HEIGHT,
        SIDEBAR_WIDTH,
        (viewport.y - TOOLBAR_HEIGHT).max(0.0),
    );
    let list = PanelRect::new(0.0, frame.y + HEAD, CONTENT, (frame.height - HEAD).max(0.0));
    (frame, list)
}

/// How far the list can scroll: what its content is taller than its box.
pub(crate) fn scroll_range(model: &SidebarModel, viewport: Vec2) -> f32 {
    let content: f32 = entries(model).iter().map(Entry::height).sum();
    (content - boxes(viewport).1.height).max(0.0)
}

/// The items a click can select, in the order they are shown, which a
/// shift-click selects a run of.
pub(crate) fn selectable(model: &SidebarModel) -> Vec<ItemId> {
    entries(model)
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Row(row, _) => match (&row.target, row.kind) {
                (RowTarget::Entity(id), kind) if !matches!(kind, RowKind::Group { .. }) => {
                    Some(ItemId::Entity(id.clone()))
                }
                (RowTarget::Entity(_) | RowTarget::Comment(_), _) => None,
            },
            Entry::Gap(_) | Entry::Canvas(_) | Entry::Rule | Entry::Head(_) | Entry::Empty => None,
        })
        .collect()
}

/// The panels of the sidebar for `model`.
pub(super) struct Built {
    pub frame: Panel,
    pub list: Panel,
}

pub(super) fn layout(ctx: &Ctx<'_>, model: &SidebarModel, viewport: Vec2) -> Built {
    let (frame_box, list_box) = boxes(viewport);
    let entries = entries(model);
    let content: f32 = entries.iter().map(Entry::height).sum();
    let range = (content - list_box.height).max(0.0);
    let scroll = ctx.ui.sidebar_scroll.clamp(0.0, range);
    let width = if range > 0.0 {
        CONTENT - SCROLLBAR
    } else {
        CONTENT
    };
    let editing = rows::editing(ctx);

    let mut nodes = Vec::new();
    let mut y = list_box.y - scroll;
    for entry in &entries {
        let band = PanelRect::new(0.0, y, width, entry.height());
        // Whole rows above and below the window are not laid out.
        if band.bottom() > list_box.y && band.y < list_box.bottom() {
            rows::entry(ctx, entry, band, list_box, editing.as_ref(), &mut nodes);
        }
        y += entry.height();
    }
    if range > 0.0 {
        nodes.push(thumb(list_box, content, scroll));
    }

    let mut head = Vec::new();
    rows::frame_head(ctx, model, frame_box, &mut head);
    Built {
        frame: Panel {
            surface: Surface::Sidebar,
            rect: frame_box,
            menu: false,
            nodes: head,
        },
        list: Panel {
            surface: Surface::SidebarList,
            rect: list_box,
            menu: false,
            nodes,
        },
    }
}

/// The scrollbar's thumb: the part of the content in the window, scaled to
/// the track.
fn thumb(window: PanelRect, content: f32, scroll: f32) -> Node {
    let share = window.height / content;
    let length = (window.height * share).max(THUMB_MIN).min(window.height);
    let travel = window.height - length;
    let range = content - window.height;
    let top = window.y
        + if range > 0.0 {
            travel * scroll / range
        } else {
            0.0
        };
    let rect = PanelRect::new(window.right() - SCROLLBAR, top, SCROLLBAR, length);
    Node {
        radius: metrics::SCROLLBAR_RADIUS,
        ..Node::fixed(rect, Chrome::Scrollbar)
    }
}

/// `text` cut to `max` pixels with an ellipsis, as `truncate` does.
pub(super) fn ellipsize(ctx: &Ctx<'_>, text: &str, max: f32) -> String {
    if max <= 0.0 {
        return String::new();
    }
    if ctx.text_width(text, TextFont::Sans) <= max {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0, chars.len());
    // The most characters that fit with the ellipsis after them.
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate: String = chars[..mid].iter().chain(&['\u{2026}']).collect();
        if ctx.text_width(&candidate, TextFont::Sans) <= max {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    let kept: String = chars[..low].iter().collect();
    format!("{}\u{2026}", kept.trim_end())
}

/// What a press on the row `pressed` of the sidebar sends, given the keys
/// held. A shift-click selects the run of rows from the one picked last, a
/// command- or control-click adds the row to the selection or takes it out,
/// and a plain click selects only it (`sidebarSelectionIntent`). Anything
/// that is not a row of the list is sent as it is.
pub(super) fn picked(
    app: &mut App,
    pressed: &ControlId,
    action: Action,
    keys: Modifiers,
) -> Action {
    let Action::Reveal { select, focus } = action else {
        return action;
    };
    if !pressed.as_str().starts_with("sidebar.") {
        return Action::Reveal { select, focus };
    }
    let order = selectable(&crate::sidebar(app));
    let position = |item: &ItemId| order.iter().position(|other| other == item);
    let current: Vec<ItemId> = app.session.selection.items().to_vec();
    let anchor = app.session.panel.anchor.clone();
    let select = match (position(&focus), anchor.as_ref().and_then(position)) {
        (Some(to), Some(from)) if keys.shift => {
            let (low, high) = (from.min(to), from.max(to));
            let mut picked = current;
            for item in &order[low..=high] {
                if !picked.contains(item) {
                    picked.push(item.clone());
                }
            }
            picked
        }
        (Some(_), _) if keys.meta || keys.control => {
            if current.contains(&focus) {
                current.into_iter().filter(|item| *item != focus).collect()
            } else {
                let mut picked = current;
                picked.push(focus.clone());
                picked
            }
        }
        _ => select,
    };
    if position(&focus).is_some() {
        app.session.panel.anchor = Some(focus.clone());
    }
    Action::Reveal { select, focus }
}
