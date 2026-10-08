//! The left sidebar, from [`SidebarModel`]: the space's canvases, then the
//! Notes and Pages of the one that is showing. A row sends its own
//! `Action`. A canvas is renamed in place, in a Kit text field.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::{Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use specular_interact::{
    Action, CanvasAction, CanvasId, CanvasRow, Event, RowKind, RowTarget, SidebarModel, SidebarRow,
};

use super::glyphs::{glyph, ink};
use super::{Rename, ShellView, focus_canvas, run};
use crate::assets::ShellIcon;
use crate::canvas;
use crate::theme;

/// A row's left edge, and how far each level of nesting moves it.
const ROW_INSET: f32 = 14.0;
const DEPTH_STEP: f32 = 14.0;
const ICON: f32 = 14.0;

fn muted() -> gpui_kit::Hsla {
    theme::tinted(theme::TEXT_MUTED)
}

fn header(label: &'static str) -> gpui_kit::Div {
    h_flex()
        .h(px(36.0))
        .px_3()
        .flex_shrink_0()
        .items_center()
        .justify_between()
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .child(label)
}

fn empty() -> impl IntoElement {
    div()
        .px(px(ROW_INSET + 8.0))
        .py_1()
        .text_size(px(11.0))
        .text_color(muted())
        .child("No items")
}

/// A row of the sidebar: flush, 28 pixels tall, filled when it is selected
/// and more faintly under the pointer.
fn row_shell(id: SharedString, selected: bool, depth: u8) -> gpui_kit::Stateful<gpui_kit::Div> {
    h_flex()
        .id(id)
        .h(px(28.0))
        .flex_shrink_0()
        .pl(px(ROW_INSET + 8.0 + f32::from(depth) * DEPTH_STEP))
        .pr_4()
        .gap_2()
        .items_center()
        .cursor_pointer()
        .when(selected, |this| this.bg(theme::tinted(theme::ROW_SELECTED)))
        .when(!selected, |this| {
            this.hover(|this| this.bg(theme::tinted(theme::ROW_HOVER)))
        })
}

fn row_icon(kind: RowKind) -> AnyElement {
    let lucide = |icon: Icon| icon.size(px(ICON)).text_color(muted()).into_any_element();
    match kind {
        RowKind::Group { .. } => lucide(Icon::new(IconName::Folder)),
        RowKind::Page => lucide(Icon::new(IconName::Globe)),
        RowKind::Text => lucide(Icon::new(ShellIcon::StickyNote)),
        RowKind::File => lucide(Icon::new(IconName::File)),
        RowKind::Drawing { .. } => lucide(Icon::new(ShellIcon::PenLine)),
        RowKind::Comment { .. } => lucide(Icon::new(ShellIcon::MessageSquare)),
        RowKind::Shape(shape) => glyph(
            specular_interact::Icon::Shape(shape),
            ink(0x0081_8181),
            None,
            false,
            ICON,
        )
        .into_any_element(),
    }
}

/// The small text at the far end of a row.
fn suffix(kind: RowKind) -> Option<String> {
    match kind {
        RowKind::Group { entity_count } => Some(entity_count.to_string()),
        RowKind::Comment { messages } if messages > 1 => Some(messages.to_string()),
        RowKind::Comment { .. }
        | RowKind::Page
        | RowKind::Text
        | RowKind::File
        | RowKind::Drawing { .. }
        | RowKind::Shape(_) => None,
    }
}

fn row_id(target: &RowTarget, section: &str) -> SharedString {
    match target {
        RowTarget::Entity(id) => format!("{section}-entity-{id}").into(),
        RowTarget::Comment(id) => format!("{section}-comment-{id}").into(),
    }
}

/// `row` and the rows inside it, each one level further in.
fn rows(row: &SidebarRow, section: &str, depth: u8, out: &mut Vec<AnyElement>) {
    let action = row.action.clone();
    let count = suffix(row.kind);
    let item = row_shell(row_id(&row.target, section), row.selected, depth)
        .when(row.dimmed, |this| this.opacity(0.5))
        .on_click(move |_, window, cx| run(&action, window, cx))
        .child(row_icon(row.kind))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(SharedString::from(row.label.clone())),
        )
        .when_some(count, |this, count| {
            this.child(div().text_color(muted()).child(SharedString::from(count)))
        });
    out.push(item.into_any_element());
    for child in &row.children {
        rows(child, section, depth.saturating_add(1), out);
    }
}

fn section(label: &'static str, list: &[SidebarRow]) -> impl IntoElement {
    let mut items = Vec::new();
    for row in list {
        rows(row, label, 0, &mut items);
    }
    v_flex()
        .child(header(label))
        .when(items.is_empty(), |this| this.child(empty()))
        .children(items)
}

impl ShellView {
    /// Opens a text field on `canvas`'s row, with its name selected.
    pub(crate) fn start_rename(
        &mut self,
        canvas: CanvasId,
        name: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let name = SharedString::from(name.to_owned());
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        input.update(cx, |state, cx| {
            state.focus(window, cx);
            state.select_all(window, cx);
        });
        // Return and a click elsewhere keep the name; Escape drops it.
        let events =
            cx.subscribe_in(
                &input,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } | InputEvent::Blur => {
                        this.finish_rename(window, cx);
                    }
                    InputEvent::Change | InputEvent::Focus => {}
                },
            );
        self.rename = Some(Rename {
            canvas,
            input,
            _events: events,
        });
        cx.notify();
    }

    /// Opens the text field on the canvas that is showing.
    pub(crate) fn rename_active(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let active = canvas::with(|canvas| {
            let active = canvas.runtime.app().space().active();
            (active.id.clone(), active.name.clone())
        });
        if let Some((id, name)) = active {
            self.start_rename(id, &name, window, cx);
        }
    }

    fn finish_rename(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(rename) = self.rename.take() else {
            return;
        };
        let name = rename.input.read(cx).value().trim().to_owned();
        if !name.is_empty() {
            canvas::dispatch(Event::Action(Action::Canvas(CanvasAction::Rename {
                canvas: Some(rename.canvas),
                name,
            })));
        }
        focus_canvas(window, cx);
        cx.notify();
    }

    fn cancel_rename(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.rename.take().is_some() {
            focus_canvas(window, cx);
            cx.notify();
        }
    }

    fn canvas_row(&self, row: &CanvasRow, cx: &mut Context<'_, Self>) -> AnyElement {
        let renaming = (self.rename.as_ref()).filter(|rename| rename.canvas == row.id);
        let id = SharedString::from(format!("canvas-{}", row.id));
        let label = match renaming {
            Some(rename) => div()
                .flex_1()
                .min_w_0()
                .child(Input::new(&rename.input).xsmall())
                .into_any_element(),
            None => div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(SharedString::from(row.label.clone()))
                .into_any_element(),
        };
        let (action, canvas, name) = (row.action.clone(), row.id.clone(), row.label.clone());
        let click = cx.listener(move |this, event: &gpui_kit::ClickEvent, window, cx| {
            if this.rename.is_some() {
                return;
            }
            if event.click_count() >= 2 {
                this.start_rename(canvas.clone(), &name, window, cx);
            } else {
                run(&action, window, cx);
            }
        });
        let view = cx.entity().downgrade();
        let (canvas, name) = (row.id.clone(), row.label.clone());
        let item = row_shell(id.clone(), renaming.is_none() && row.active, 0)
            .on_click(click)
            .child(Icon::new(IconName::File).size(px(ICON)).text_color(muted()))
            .child(label)
            .when(row.active && renaming.is_none(), |this| {
                this.child(Icon::new(IconName::Check).size(px(ICON)))
            });
        div()
            .id(SharedString::from(format!("{id}-menu")))
            // Escape in the field drops the rename before the field sees it.
            .capture_action(cx.listener(
                |this, _: &gpui_kit::component::input::Escape, window, cx| {
                    this.cancel_rename(window, cx);
                },
            ))
            .child(item)
            .context_menu(move |menu, _, _| {
                let (view, canvas, name) = (view.clone(), canvas.clone(), name.clone());
                let delete = Action::Canvas(CanvasAction::Delete(Some(canvas.clone())));
                menu.item(
                    PopupMenuItem::new("Rename canvas").on_click(move |_, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.start_rename(canvas.clone(), &name, window, cx);
                        });
                    }),
                )
                .item(
                    PopupMenuItem::new("Delete canvas")
                        .on_click(move |_, window, cx| run(&delete, window, cx)),
                )
            })
            .into_any_element()
    }

    /// The sidebar for `model`.
    pub(super) fn sidebar(
        &mut self,
        model: &SidebarModel,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        let canvases: Vec<AnyElement> = (model.canvases.iter())
            .map(|row| self.canvas_row(row, cx))
            .collect();
        let new_canvas = Action::Canvas(CanvasAction::New);
        v_flex()
            .id("sidebar")
            .occlude()
            .w(px(theme::SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .pt(px(theme::TOOLBAR_HEIGHT))
            .bg(theme::solid(theme::PANEL))
            .border_r_1()
            .border_color(theme::solid(theme::CHROME_BORDER))
            .child(
                v_flex()
                    .id("sidebar-rows")
                    .size_full()
                    .overflow_y_scroll()
                    .child(
                        header("Canvases").child(
                            Button::new("new-canvas")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Plus)
                                .tooltip("New canvas")
                                .on_click(move |_, window, cx| run(&new_canvas, window, cx)),
                        ),
                    )
                    .children(canvases)
                    .child(
                        div()
                            .mt_1()
                            .h(px(1.0))
                            .flex_shrink_0()
                            .bg(theme::solid(theme::PANEL_BORDER)),
                    )
                    .child(section("Notes", &model.notes))
                    .child(section("Pages", &model.pages)),
            )
    }
}
