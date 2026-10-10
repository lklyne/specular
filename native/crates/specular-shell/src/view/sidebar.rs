//! The left sidebar, from [`SidebarModel`]: the space's canvases, then the
//! Notes and Pages of the one that is showing.
//!
//! Everything it shows and does is the model's: whether it is there at all,
//! which sections are folded and which rows are open, each row's glyph and
//! far-end text, and the `Action` a click sends. The view keeps only what a
//! renderer must: the scroll offset, the row a shift-click runs from, and
//! the Kit text field of a canvas being renamed.
//!
//! The sizes are `left-sidebar/App.tsx`'s, as the built-in renderer has
//! them in `panel/builtin/sidebar/metrics.rs`.

use gpui_kit::component::menu::ContextMenuExt as _;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement, Modifiers,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    px,
};
use specular_interact::{
    Action, CanvasRow, ControlId, Icon, MenuTarget, SectionHead, SidebarModel, SidebarRow,
    context_menu,
};

use super::glyphs::{glyph, ink};
use super::named::mark;
use super::{ShellView, field, menu, pick, run};
use crate::canvas;
use crate::theme;

/// A head: `h-9 px-3`, its chevron `gap-1.5` after the title.
const HEAD: f32 = 36.0;
const HEAD_PAD: f32 = 12.0;
/// The add button: `p-1.5` around a 14 px glyph.
const ADD: f32 = 26.0;
/// A row: 28 px tall, 22 in from the left and 16 from the right, 14 more
/// for each level of depth.
const ROW: f32 = 28.0;
const PAD_LEFT: f32 = 22.0;
const PAD_RIGHT: f32 = 16.0;
const STEP: f32 = 14.0;
/// A row's glyph, and a fold chevron 16 px left of the row's content.
const ICON: f32 = 14.0;
const FOLD: f32 = 12.0;
const FOLD_LEFT: f32 = 16.0;

fn muted() -> gpui_kit::Hsla {
    theme::tinted(theme::text_muted())
}

fn element_id(id: &ControlId) -> SharedString {
    SharedString::from(id.as_str().to_owned())
}

fn small(icon: Icon, tone: u32, size: f32) -> impl IntoElement {
    glyph(icon, ink(tone), None, false, size)
}

/// A section's head: its title and a chevron, which fold it.
fn head(model: &SectionHead) -> impl IntoElement {
    let toggle = model.toggle.clone();
    let chevron = if model.folded {
        Icon::ChevronRight
    } else {
        Icon::ChevronDown
    };
    h_flex()
        .id(element_id(&model.id))
        .h(px(HEAD))
        .flex_1()
        .min_w_0()
        .gap_1p5()
        .items_center()
        .cursor_pointer()
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(SharedString::from(model.title.clone())),
        )
        .child(small(chevron, theme::text(), FOLD))
        .child(mark(&model.id))
        .on_click(move |_, window, cx| run(&toggle, window, cx))
}

/// A row of the list: filled when it is selected, and more faintly under
/// the pointer.
fn row_shell(id: &ControlId, selected: bool, depth: u8) -> gpui_kit::Stateful<gpui_kit::Div> {
    h_flex()
        .id(element_id(id))
        .relative()
        .h(px(ROW))
        .flex_shrink_0()
        .pl(px(PAD_LEFT + f32::from(depth) * STEP))
        .pr(px(PAD_RIGHT))
        .gap_2()
        .items_center()
        .cursor_pointer()
        .when(selected, |this| {
            this.bg(theme::tinted(theme::row_selected()))
        })
        .when(!selected, |this| {
            this.hover(|this| this.bg(theme::tinted(theme::row_hover())))
        })
        .child(mark(id))
}

fn label(text: &str) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .truncate()
        .child(SharedString::from(text.to_owned()))
}

/// The chevron that opens and closes the rows inside a row.
fn fold(row: &SidebarRow, toggle: &Action, open: bool) -> impl IntoElement {
    let toggle = toggle.clone();
    let chevron = if open {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    div()
        .id(element_id(&row.id.child("toggle")))
        .absolute()
        .left(px(-FOLD_LEFT - 2.0))
        .top(px((ICON - ROW) / 2.0))
        .w(px(FOLD + 4.0))
        .h(px(ROW))
        .flex()
        .items_center()
        .justify_center()
        .child(small(chevron, theme::glyph_muted(), FOLD))
        .child(mark(&row.id.child("toggle")))
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            run(&toggle, window, cx);
        })
}

impl ShellView {
    /// What a click on a row sends with `keys` held: the row's own action,
    /// its selection widened or narrowed as the keys ask.
    fn picked(&mut self, action: &Action, keys: Modifiers) -> Action {
        let Action::Reveal { select, focus } = action else {
            return action.clone();
        };
        let order =
            canvas::models().map_or_else(Vec::new, |models| pick::selectable(&models.sidebar));
        let current = canvas::with(|canvas| {
            let selection = &canvas.runtime.app().session().selection;
            selection.items().to_vec()
        })
        .unwrap_or_default();
        let anchor = self.picked_last.as_ref();
        let select = pick::picked(&order, &current, anchor, focus, select.clone(), keys);
        if order.contains(focus) {
            self.picked_last = Some(focus.clone());
        }
        Action::Reveal {
            select,
            focus: focus.clone(),
        }
    }

    /// `row` and, while it is open, the rows inside it.
    fn rows(row: &SidebarRow, depth: u8, out: &mut Vec<AnyElement>, cx: &mut Context<'_, Self>) {
        let action = row.action.clone();
        let click = cx.listener(move |this, event: &ClickEvent, window, cx| {
            let action = this.picked(&action, event.modifiers());
            run(&action, window, cx);
        });
        let trailing = row.trailing.clone().map(SharedString::from);
        // The chevron hangs off the glyph's left edge.
        let folds = row.toggle.as_ref().zip(row.expanded);
        let glyph_box = div()
            .relative()
            .flex_shrink_0()
            .child(small(row.glyph, theme::glyph_muted(), ICON))
            .when_some(folds, |this, (toggle, open)| {
                this.child(fold(row, toggle, open))
            });
        let item = row_shell(&row.id, row.selected, depth)
            .when(row.dimmed, |this| this.opacity(0.5))
            .on_click(click)
            .child(glyph_box)
            .child(label(&row.label))
            .when_some(trailing, |this, text| {
                this.child(div().flex_shrink_0().text_color(muted()).child(text))
            });
        out.push(item.into_any_element());
        if row.expanded == Some(true) {
            for child in &row.children {
                Self::rows(child, depth.saturating_add(1), out, cx);
            }
        }
    }

    fn canvas_row(row: &CanvasRow, window: &mut Window, cx: &mut Context<'_, Self>) -> AnyElement {
        let renaming = field::is_inline(&row.rename.id, cx);
        let name = if renaming {
            div()
                .flex_1()
                .min_w_0()
                .child(field::input(&row.rename, false, window, cx))
                .into_any_element()
        } else {
            label(&row.label).into_any_element()
        };
        let (action, rename) = (row.action.clone(), row.rename.clone());
        let item = row_shell(&row.control, false, 0)
            .on_click(move |event: &ClickEvent, window, cx| {
                if field::is_inline(&rename.id, cx) {
                    return;
                }
                if event.click_count() >= 2 {
                    field::begin_inline(&rename, window, cx);
                } else {
                    run(&action, window, cx);
                }
            })
            .child(small(Icon::File, theme::glyph_muted(), ICON))
            .child(name)
            .when(row.active && !renaming, |this| {
                this.child(small(Icon::Check, theme::text(), ICON))
            });
        let target = MenuTarget::Canvas(row.id.clone());
        div()
            .id(element_id(&row.control.child("menu")))
            .child(item)
            .context_menu(move |popup, _, _| {
                let model = canvas::with(|canvas| context_menu(canvas.runtime.app(), &target));
                match model.flatten() {
                    Some(model) => menu::filled(popup, &model),
                    None => popup,
                }
            })
            .into_any_element()
    }

    /// The sidebar for `model`, over the canvas's left edge under the
    /// toolbar. The app counts that strip as covered whenever the model is
    /// visible, which is exactly when this is drawn.
    pub(super) fn sidebar(
        model: &SidebarModel,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        let add = model.add_canvas.clone();
        let folded = model.canvases_head.folded;
        let mut list = v_flex()
            .id("sidebar-rows")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        if !folded {
            let canvases: Vec<AnyElement> = (model.canvases.iter())
                .map(|row| Self::canvas_row(row, window, cx))
                .collect();
            list = list.child(v_flex().flex_shrink_0().pt_0p5().pb_2().children(canvases));
        }
        let mut sections = v_flex().flex_shrink_0().py_2();
        for (section, rows) in [
            (&model.notes_head, &model.notes),
            (&model.pages_head, &model.pages),
        ] {
            sections = sections.child(h_flex().px(px(HEAD_PAD)).child(head(section)));
            if !section.folded {
                let mut items = Vec::new();
                for row in rows {
                    Self::rows(row, 0, &mut items, cx);
                }
                sections = sections.children(items);
            }
        }
        if model.is_empty() {
            sections = sections.child(
                div()
                    .px(px(PAD_LEFT))
                    .py_1()
                    .text_color(muted())
                    .child("No items"),
            );
        }
        let rule = || {
            div()
                .h(px(1.0))
                .flex_shrink_0()
                .bg(theme::solid(theme::panel_border()))
        };
        v_flex()
            .id("sidebar")
            .occlude()
            .absolute()
            .left_0()
            .top(px(theme::CHROME_HEIGHT))
            .bottom_0()
            .w(px(theme::SIDEBAR_WIDTH))
            .bg(theme::solid(theme::panel()))
            .border_r_1()
            .border_color(theme::solid(theme::chrome_border()))
            .child(
                h_flex()
                    .h(px(HEAD))
                    .flex_shrink_0()
                    .px(px(HEAD_PAD))
                    .gap_1()
                    .items_center()
                    .child(head(&model.canvases_head))
                    .child(
                        div()
                            .id("sidebar.add")
                            .size(px(ADD))
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .justify_center()
                            .rounded(px(8.0))
                            .cursor_pointer()
                            .hover(|this| this.bg(theme::tinted(theme::row_selected())))
                            .tooltip(crate::tip::view("New canvas"))
                            .child(small(Icon::Plus, theme::text(), ICON))
                            .child(mark(&ControlId::new("sidebar.add")))
                            .on_click(move |_, window, cx| run(&add, window, cx)),
                    ),
            )
            .when(folded, |this| this.child(rule()))
            .child(list.child(rule()).child(sections))
    }
}
