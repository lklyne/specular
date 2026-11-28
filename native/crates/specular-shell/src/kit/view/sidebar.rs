//! The left sidebar, from [`SidebarModel`]: the projects' canvases, each
//! project under a head of its own when there are several (ADR 0062), then
//! the section mods (`"section": "sidebar"`), such as Notes and Pages.
//!
//! Everything it shows and does is the model's: whether it is there at all,
//! which sections are folded and which rows are open, each row's glyph and
//! far-end text, and the `Action` a click sends. The view keeps only what a
//! renderer must: the scroll offset and the Kit text field of a canvas
//! being renamed. A row is pressed by name, so the app keeps the row a
//! shift-click runs from.
//!
//! The sizes are `left-sidebar/App.tsx`'s.

use gpui_kit::component::menu::ContextMenuExt as _;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    px,
};
use specular_interact::{
    Action, CanvasRow, ControlId, Icon, MenuTarget, ProjectAction, ProjectSection, SectionHead,
    SidebarButton, SidebarModel, context_menu,
};

use super::glyphs::{glyph, ink};
use super::named::mark;
use super::{ShellView, field, menu, run};
use crate::canvas;
use crate::theme;

/// The title bar's height, the tab row's, and the room the traffic lights
/// take at its left.
const TITLE: f32 = 38.0;
const TRAFFIC_LIGHTS: f32 = 86.0;
/// The toggle on the title bar, as the tab row's: a 28 px box, a 14 px glyph.
const TITLE_BUTTON: f32 = 28.0;
const TITLE_GLYPH: f32 = 14.0;
/// A head: `h-9 px-3`, its chevron `gap-1.5` after the title.
const HEAD: f32 = 36.0;
const HEAD_PAD: f32 = 12.0;
/// The add button: `p-1.5` around a 14 px glyph.
const ADD: f32 = 26.0;
/// A project head's buttons, smaller than the add button so the pair sits
/// close.
const HEAD_BUTTON: f32 = 22.0;
/// A row: 28 px tall, 22 in from the left and 16 from the right.
const ROW: f32 = 28.0;
const PAD_LEFT: f32 = 22.0;
const PAD_RIGHT: f32 = 16.0;
/// A row's glyph, and a fold chevron.
const ICON: f32 = 14.0;
const FOLD: f32 = 12.0;

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
        .gap(crate::theme::space(1.5))
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

/// A row of the list, filled faintly under the pointer.
fn row_shell(id: &ControlId) -> gpui_kit::Stateful<gpui_kit::Div> {
    h_flex()
        .id(element_id(id))
        .relative()
        .h(px(ROW))
        .flex_shrink_0()
        .pl(px(PAD_LEFT))
        .pr(px(PAD_RIGHT))
        .gap(crate::theme::space(2.0))
        .items_center()
        .cursor_pointer()
        .hover(|this| this.bg(theme::tinted(theme::row_hover())))
        .child(mark(id))
}

fn label(text: &str) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .truncate()
        .child(SharedString::from(text.to_owned()))
}

impl ShellView {
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
        let item = row_shell(&row.control)
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
                    Some(model) => menu::filled(popup, &model, |_, action, window, cx| {
                        run(action, window, cx);
                    }),
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
        let sections = crate::mods::sidebar_sections();
        // With no section mods the canvases are all the sidebar holds, so
        // they need no head of their own: each project's head has the + for
        // a new canvas in it.
        let titled = !sections.is_empty();
        let folded = titled && model.canvases_head.folded;
        let mut list = v_flex()
            .id("sidebar-rows")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        if !folded {
            let mut canvases: Vec<AnyElement> = Vec::new();
            if model.projects.len() > 1 || (!titled && !model.projects.is_empty()) {
                for (n, project) in model.projects.iter().enumerate() {
                    let mut group = v_flex()
                        .flex_shrink_0()
                        .when(n > 0, |this| this.pt(crate::theme::space(3.0)))
                        .child(project_head(project));
                    if !project.folded {
                        group = group.children(
                            (project.canvases.iter()).map(|row| Self::canvas_row(row, window, cx)),
                        );
                    }
                    canvases.push(group.into_any_element());
                }
            } else {
                canvases
                    .extend((model.canvases.iter()).map(|row| Self::canvas_row(row, window, cx)));
            }
            list = list.child(
                v_flex()
                    .flex_shrink_0()
                    .pt(crate::theme::space(if titled { 0.5 } else { 2.0 }))
                    .pb(crate::theme::space(2.0))
                    .children(canvases),
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
            .size_full()
            .bg(super::material::panel_fill(theme::panel()))
            .border_r(crate::theme::line())
            .border_color(theme::solid(theme::chrome_border()))
            .children(model.title.as_ref().map(title_bar))
            .when(titled, |this| this.child(canvases_head(model)))
            .when(folded, |this| this.child(rule()))
            .child(list.when(titled, |this| {
                this.child(rule()).child(
                    v_flex()
                        .flex_shrink_0()
                        .py(crate::theme::space(2.0))
                        .children(sections),
                )
            }))
            .child(rule())
            .child(footer())
    }
}

/// The sidebar's top while it reaches the window's top edge: bare title
/// bar to drag the window by, with the traffic lights and then the
/// sidebar's own toggle on it.
fn title_bar(toggle: &SidebarButton) -> impl IntoElement {
    div()
        .relative()
        .h(px(TITLE))
        .flex_shrink_0()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(super::tabs::window_drag()),
        )
        .child(
            h_flex()
                .absolute()
                .top_0()
                .left(px(TRAFFIC_LIGHTS))
                .h(px(TITLE))
                .items_center()
                .child(title_toggle(toggle)),
        )
}

/// The sidebar's toggle on its title bar: the same box and glyph as the
/// tab row's mod draws while the sidebar is hidden, so it does not change
/// as the sidebar opens and closes.
fn title_toggle(toggle: &SidebarButton) -> impl IntoElement {
    let action = toggle.action.clone();
    div()
        .id(element_id(&toggle.id))
        .size(px(TITLE_BUTTON))
        .flex()
        .items_center()
        .justify_center()
        .rounded(crate::theme::corner(8.0))
        .cursor_pointer()
        .hover(|this| this.bg(theme::tinted(theme::row_selected())))
        .tooltip(crate::tip::view(toggle.label.to_string()))
        .child(glyph(
            toggle.icon,
            ink(theme::toolbar_text()),
            None,
            false,
            TITLE_GLYPH,
        ))
        .child(mark(&toggle.id))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The foot of the sidebar: a button that opens a folder or a repo as a
/// project.
fn footer() -> impl IntoElement {
    let id = ControlId::new("sidebar.open");
    let open = Action::Project(ProjectAction::Open);
    h_flex()
        .h(px(HEAD))
        .flex_shrink_0()
        .px(px(HEAD_PAD))
        .items_center()
        .child(
            h_flex()
                .id(element_id(&id))
                .h(px(ADD))
                .px(crate::theme::space(1.5))
                .gap(crate::theme::space(1.5))
                .items_center()
                .rounded(crate::theme::corner(8.0))
                .cursor_pointer()
                .text_color(muted())
                .hover(|this| this.bg(theme::tinted(theme::row_selected())))
                .tooltip(crate::tip::view("Open a folder or a repo as a project"))
                .child(small(Icon::Plus, theme::text_muted(), ICON))
                .child("New project")
                .child(mark(&id))
                .on_click(move |_, window, cx| run(&open, window, cx)),
        )
}

/// The canvases' head and its button for a new canvas.
fn canvases_head(model: &SidebarModel) -> impl IntoElement {
    let add = model.add_canvas.clone();
    h_flex()
        .h(px(HEAD))
        .flex_shrink_0()
        .px(px(HEAD_PAD))
        .gap(crate::theme::space(1.0))
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
                .rounded(crate::theme::corner(8.0))
                .cursor_pointer()
                .hover(|this| this.bg(theme::tinted(theme::row_selected())))
                .tooltip(crate::tip::view("New canvas"))
                .child(small(Icon::Plus, theme::text(), ICON))
                .child(mark(&ControlId::new("sidebar.add")))
                .on_click(move |_, window, cx| run(&add, window, cx)),
        )
}

/// A project's head in the canvases list: its name and branch, which fold
/// it. Under the pointer it shows a button for a new canvas in it and, at
/// the far right, the fold chevron.
/// A button on a project's head: a glyph in a box filled under the pointer.
fn head_button(id: &ControlId) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(element_id(id))
        .size(px(HEAD_BUTTON))
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(crate::theme::corner(6.0))
        .hover(|this| this.bg(theme::tinted(theme::row_selected())))
}

fn project_head(project: &ProjectSection) -> impl IntoElement {
    const HOVERED: &str = "project-head";
    let toggle = project.toggle.clone();
    let add = project.add_canvas.clone();
    let add_id = project.control.child("add");
    let title = match &project.branch {
        Some(branch) => format!("{} \u{b7} {branch}", project.label),
        None => project.label.clone(),
    };
    let chevron = if project.folded {
        Icon::ChevronRight
    } else {
        Icon::ChevronDown
    };
    h_flex()
        .id(element_id(&project.control))
        .group(HOVERED)
        .h(px(ROW))
        .flex_shrink_0()
        .pl(px(HEAD_PAD))
        .pr(px(HEAD_PAD))
        .gap(crate::theme::space(1.0))
        .items_center()
        .cursor_pointer()
        .text_color(if project.active {
            theme::tinted(theme::text())
        } else {
            muted()
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(SharedString::from(title)),
        )
        .child(
            h_flex()
                .flex_shrink_0()
                .gap(crate::theme::space(0.5))
                .invisible()
                .group_hover(HOVERED, gpui_kit::Styled::visible)
                .child(
                    head_button(&add_id)
                        .tooltip(crate::tip::view("New canvas in this project"))
                        .child(small(Icon::Plus, theme::text_muted(), ICON))
                        .child(mark(&add_id))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            run(&add, window, cx);
                        }),
                )
                .child(head_button(&project.control.child("fold")).child(small(
                    chevron,
                    theme::text_muted(),
                    FOLD,
                ))),
        )
        .child(mark(&project.control))
        .on_click(move |_, window, cx| run(&toggle, window, cx))
}
