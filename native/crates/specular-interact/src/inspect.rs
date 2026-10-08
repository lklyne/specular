//! The inspect tool: the DOM node under the pointer, and the one picked.
//!
//! A hover over page content asks the page for its node there
//! ([`Effect::InspectAt`](crate::Effect::InspectAt)) and a click asks with
//! `pick`. The answers are session state: the hovered node shows an outline
//! and a popover while the tool is in hand, and the picked node is the
//! chat's turn target until it is cleared.

use glam::Vec2;
use specular_core::InspectedNode;
use specular_doc::EntityId;

use crate::anchor::matches_page_url;
use crate::{App, Effect, Focus, Hit, PageNotice, ScreenRect, Tool, geometry};

mod popover;

/// A node of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectTarget {
    /// The page entity.
    pub page: EntityId,
    /// The node.
    pub node: InspectedNode,
}

/// What the inspect tool holds.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct InspectState {
    /// The node under the pointer.
    pub(crate) hovered: Option<InspectTarget>,
    /// The node a click picked.
    pub(crate) selected: Option<InspectTarget>,
    /// The page and point of the last hover question, so a move within one
    /// CSS pixel asks nothing and a late answer is told from the current one.
    pub(crate) asked: Option<(EntityId, Vec2)>,
    /// The address the picked node's page showed when it was picked, so a
    /// navigation drops the pick.
    pub(crate) selected_url: Option<String>,
}

/// What the canvas draws for the inspect tool: an outline on the node and a
/// popover beside it (`InspectPopoverLayer.tsx`).
#[derive(Debug, Clone, PartialEq)]
pub struct InspectModel {
    /// The node's box, in screen pixels.
    pub outline: ScreenRect,
    /// The popover.
    pub popover: InspectPopover,
}

/// The inspect popover: the node's tag, size and key styles.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectPopover {
    /// Where it sits, in screen pixels, placed as
    /// `inspect-popover-position.ts` places it.
    pub rect: ScreenRect,
    /// The tag name, drawn on a blue chip.
    pub tag: String,
    /// `#id.class.class`: the id and up to three classes. Empty with none.
    pub remainder: String,
    /// The node's size in the page's CSS pixels, as `160 × 48`.
    pub size: String,
    /// The font line. `None` when the page reported no font family.
    pub font: Option<InspectFont>,
    /// The text colour, then the background when it is not transparent.
    pub swatches: Vec<InspectSwatch>,
}

/// The popover's font line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectFont {
    /// The first family of the computed `font-family`, unquoted.
    pub family: String,
    /// `14px · 400`: the size and weight that were reported.
    pub detail: String,
}

/// One colour of the popover's last line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSwatch {
    /// `text` or `bg`.
    pub label: String,
    /// The computed colour, as CSS gives it (`rgb(17, 24, 39)`).
    pub value: String,
}

impl App {
    /// What the inspect tool shows now: the hovered node, else the picked
    /// one, while the tool is in hand and the node's page is on the canvas.
    pub fn inspect(&self) -> Option<InspectModel> {
        if self.session.tool != Tool::Inspect {
            return None;
        }
        let state = &self.session.inspect;
        let target = state.hovered.as_ref().or(state.selected.as_ref())?;
        let placement = self.page_placement(&target.page)?;
        let css = target.node.bounding_box;
        let canvas = geometry::rect(
            placement.to_canvas(glam::DVec2::new(f64::from(css.x), f64::from(css.y))),
            glam::DVec2::new(f64::from(css.width), f64::from(css.height))
                * placement.canvas_per_css(),
        );
        let outline = ScreenRect::of(&self.session.camera, canvas);
        let popover = InspectPopover::of(&target.node, outline, self.session.viewport);
        Some(InspectModel { outline, popover })
    }

    /// The node a click of the inspect tool picked: the chat's turn target.
    pub fn inspected(&self) -> Option<&InspectTarget> {
        self.session.inspect.selected.as_ref()
    }
}

/// The address `page` shows: what it last reported, else the entity's.
pub(crate) fn address(app: &App, page: &EntityId) -> Option<String> {
    let live = app.page_state(page).and_then(|state| state.url.clone());
    live.or_else(|| {
        let entity = app.document.entity(page)?;
        Some(crate::app::page_of(entity)?.url.clone())
    })
}

/// The pointer is over `hit` with the tool in hand and nothing dragging:
/// asks the page for its node there when the point is a new one.
pub(crate) fn hover(app: &mut App, hit: &Hit, effects: &mut Vec<Effect>) {
    let Hit::PageContent { page, local } = hit else {
        clear_hover(app);
        return;
    };
    let point = local.floor();
    let state = &mut app.session.inspect;
    if state
        .asked
        .as_ref()
        .is_some_and(|(p, at)| p == page && *at == point)
    {
        return;
    }
    state.asked = Some((page.clone(), point));
    effects.push(Effect::InspectAt {
        page: page.clone(),
        point,
        pick: false,
    });
}

/// The pointer left page content: nothing is hovered or being asked.
pub(crate) fn clear_hover(app: &mut App) {
    let state = &mut app.session.inspect;
    state.hovered = None;
    state.asked = None;
}

/// A left press with the inspect tool. Returns whether the tool took it:
/// only a press on page content, which asks for the node to pick.
pub(crate) fn press(hit: &Hit, effects: &mut Vec<Effect>) -> bool {
    let Hit::PageContent { page, local } = hit else {
        return false;
    };
    effects.push(Effect::InspectAt {
        page: page.clone(),
        point: local.floor(),
        pick: true,
    });
    true
}

/// A page answered.
pub(crate) fn on_notice(app: &mut App, page: &EntityId, notice: &PageNotice) {
    match notice {
        PageNotice::Inspected { point, pick, node } => {
            let target = node.as_deref().map(|node| InspectTarget {
                page: page.clone(),
                node: node.clone(),
            });
            if *pick {
                if let Some(target) = target {
                    app.session.inspect.selected_url = address(app, page);
                    app.session.inspect.hovered = Some(target.clone());
                    app.session.inspect.selected = Some(target);
                }
                return;
            }
            let state = &mut app.session.inspect;
            let current = app.session.tool == Tool::Inspect
                && state.asked.as_ref() == Some(&(page.clone(), *point));
            if current {
                state.hovered = target;
            }
        }
        PageNotice::Url(url) => {
            let state = &mut app.session.inspect;
            if state.selected.as_ref().is_none_or(|s| &s.page != page) {
                return;
            }
            if matches_page_url(state.selected_url.as_deref(), Some(url)) {
                state.selected_url.get_or_insert_with(|| url.clone());
            } else {
                state.selected = None;
                state.selected_url = None;
            }
        }
        PageNotice::Loaded { .. }
        | PageNotice::Crashed { .. }
        | PageNotice::Title(_)
        | PageNotice::Loading { .. }
        | PageNotice::Scrolled { .. }
        | PageNotice::ScrollProgress { .. }
        | PageNotice::Pointed { .. }
        | PageNotice::Candidates { .. }
        | PageNotice::ImeCompositionBounds(_)
        | PageNotice::DevtoolsUrl(_) => {}
    }
}

/// The tool changed: the hover goes with the tool and the pick stays, as
/// the chat's pill.
pub(crate) fn on_tool_change(app: &mut App) {
    if app.session.tool != Tool::Inspect {
        clear_hover(app);
    }
}

/// Drops what is on a page that is gone, or on a canvas that was left.
pub(crate) fn settle(app: &mut App, switched: bool) {
    let gone = |app: &App, target: &InspectTarget| {
        switched
            || (app.document.entity(&target.page))
                .and_then(crate::app::page_of)
                .is_none()
    };
    let state = &app.session.inspect;
    if state.selected.as_ref().is_some_and(|t| gone(app, t)) {
        let state = &mut app.session.inspect;
        state.selected = None;
        state.selected_url = None;
    }
    if app
        .session
        .inspect
        .hovered
        .as_ref()
        .is_some_and(|t| gone(app, t))
    {
        clear_hover(app);
    }
}

/// Escape with the select tool and nothing else in flight drops the picked
/// node. Returns whether it did.
pub(crate) fn cancel(app: &mut App) -> bool {
    let session = &app.session;
    let idle = session.tool == Tool::Select
        && session.gesture.is_none()
        && session.editing.is_none()
        && session.focus == Focus::Canvas;
    if !idle || session.inspect.selected.is_none() {
        return false;
    }
    let state = &mut app.session.inspect;
    state.selected = None;
    state.selected_url = None;
    true
}
