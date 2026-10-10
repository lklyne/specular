//! The window's content: the Kit's chrome and sidebar, and the slot the
//! canvas shows through.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────┐
//! │ tab row (Kit): the title bar, the zoom, the panel     │
//! ├──────────┬───┬───────────────────────────┬────────────┤
//! │ sidebar  │ t │ dock (Kit), while it has  │ right      │
//! │ (Kit)    │ o │ something to hold         │ panel      │
//! │          │ o ├───────────────────────────┤ (Kit)      │
//! │          │ l │ canvas slot: nothing      │            │
//! │          │ s │ painted, so the canvas    │            │
//! │          │   │ view under GPUI shows     │            │
//! └──────────┴───┴───────────────────────────┴────────────┘
//! ```
//!
//! The slot fills the window beside the right panel, as the app's own
//! layout assumes, and everything else lies over it. The app counts as
//! covered the tab row (`CHROME_HEIGHT`), the sidebar while its model says
//! it is visible, and the tools while they are docked
//! (`App::covered_left`). The tools can float over the slot's top or bottom
//! instead, covering nothing the app counts.
//!
//! The dock is a bar over the top of the slot. In a tab that shows its
//! item it is always there, holding the lens, and the app counts it
//! (`App::covered_top`). On the canvas it comes with a selection and goes
//! with it, over the canvas, so nothing under it moves. The right panel is
//! beside the slot, so opening it or dragging its edge makes the app's
//! viewport narrower.

mod chat;
mod controls;
mod dock;
mod dropdown;
mod field;
mod glyphs;
mod ime;
mod menu;
mod named;
mod onboarding;
mod pick;
mod sidebar;
mod slot;
mod tabs;
mod toolbar;

use std::path::PathBuf;
use std::rc::Rc;

use glam::Vec2;
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Context, ExternalPaths, FocusHandle, Global, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Styled as _, Window, div, px,
};
use specular_doc::ItemId;
use specular_interact::{Action, CanvasAction, Event, SidebarAction, TOOLS_DOCK_WIDTH, ToolsPlace};

pub(crate) use self::named::shown as shown_controls;
use self::slot::Pointer;
use crate::canvas::{self, Models};
use crate::surface::WindowAsks;
use crate::theme;

/// The canvas slot's focus, so a click on a Kit control can hand the keys
/// back to the canvas.
pub(crate) struct CanvasFocus(pub(crate) FocusHandle);

impl Global for CanvasFocus {}

/// Runs `action` through `update` and gives the keys back to the canvas.
///
/// Renaming a canvas is the one action a renderer answers itself: the model
/// asks for the row's name field to be typed in, and here that field is the
/// Kit's.
pub(crate) fn run(action: &Action, window: &mut Window, cx: &mut App) {
    if let Action::Canvas(CanvasAction::BeginRename(canvas)) = action {
        begin_rename(canvas.as_ref(), window, cx);
        return;
    }
    canvas::dispatch(Event::Action(action.clone()));
    focus_canvas(window, cx);
}

/// Opens the name field on the row of `canvas`, or of the canvas showing,
/// with the sidebar shown first if it was hidden.
fn begin_rename(canvas: Option<&specular_interact::CanvasId>, window: &mut Window, cx: &mut App) {
    if canvas::models().is_some_and(|models| !models.sidebar.visible) {
        canvas::dispatch(Event::Action(Action::Sidebar(SidebarAction::Toggle)));
    }
    let Some(models) = canvas::models() else {
        return;
    };
    let row = (models.sidebar.canvases.iter()).find(|row| match canvas {
        Some(canvas) => row.id == *canvas,
        None => row.active,
    });
    if let Some(row) = row {
        field::begin_inline(&row.rename, window, cx);
    }
}

/// Files dropped on the window at `at`, in points from the content's
/// corner: the end of a drag from Finder, and a script's `drop` step.
pub(crate) fn drop_files(paths: Vec<PathBuf>, at: Vec2) {
    canvas::with(|canvas| {
        let origin = (canvas.runtime.window())
            .map_or(Vec2::ZERO, crate::surface::CanvasSurface::slot_origin);
        canvas.runtime.drop_files(paths, Some(at - origin));
    });
}

/// How much of the slot's left edge the Kit covers: the sidebar, and the
/// tools while they are docked.
fn covered_left(models: &Models) -> f32 {
    let sidebar = if models.sidebar.visible {
        theme::SIDEBAR_WIDTH
    } else {
        0.0
    };
    let tools = if models.toolbar.tools == ToolsPlace::Docked {
        TOOLS_DOCK_WIDTH
    } else {
        0.0
    };
    sidebar + tools
}

/// What lies over the slot and moves with its edges: the dock's bar, and
/// the tools while they float.
fn over_slot(models: &Models, window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
    let left = covered_left(models);
    let dock = dock::dock(models.dock.as_ref(), &models.toolbar, left, window, cx);
    let mut top = theme::CHROME_HEIGHT;
    if dock.is_some() {
        top += theme::DOCK_ROW;
    }
    let mut out: Vec<AnyElement> = dock
        .into_iter()
        .map(IntoElement::into_any_element)
        .collect();
    if models.toolbar.tools != ToolsPlace::Docked {
        let shown = models.tool_options.as_ref();
        out.push(
            toolbar::floating(&models.toolbar, shown, (left, top), window, cx).into_any_element(),
        );
    }
    out
}

/// Gives the keys back to the canvas.
pub(crate) fn focus_canvas(window: &mut Window, cx: &mut App) {
    if let Some(focus) = cx.try_global::<CanvasFocus>().map(|focus| focus.0.clone()) {
        window.focus(&focus, cx);
    }
}

/// The root view: it holds what only a view can (focus, the input method's
/// marked text, the sidebar row a shift-click runs from) and draws
/// everything else from the models.
pub(crate) struct ShellView {
    focus: FocusHandle,
    asks: Rc<WindowAsks>,
    pointer: Rc<Pointer>,
    /// The input method's marked text, which GPUI asks back for.
    marked: String,
    /// The item last picked in the sidebar.
    picked_last: Option<ItemId>,
    /// What the right panel keeps between frames.
    chat: chat::ChatUi,
}

impl ShellView {
    pub(crate) fn new(
        asks: Rc<WindowAsks>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        cx.set_global(CanvasFocus(focus.clone()));
        window.focus(&focus, cx);
        Self {
            focus,
            asks,
            pointer: Rc::new(Pointer::default()),
            marked: String::new(),
            picked_last: None,
            chat: chat::ChatUi::new(window, cx),
        }
    }
}

impl ShellView {
    /// Whether the keys are in one of the Kit's text fields: the composer,
    /// or a model field being typed in (a canvas's name, a dock field).
    pub(crate) fn typing(&self, window: &Window, cx: &mut App) -> bool {
        let in_field = window
            .focused(cx)
            .is_some_and(|focused| field::holds_focus(&focused, window, cx));
        in_field || self.composer_focused(window, cx)
    }
}

impl Render for ShellView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        // What effects asked of the window is applied by this render.
        self.asks.changed.set(false);
        if let Some(wanted) = self.asks.field.take() {
            field::want(wanted, cx);
        }
        named::begin_frame();
        let models = canvas::models();
        if let Some(model) = models
            .as_ref()
            .and_then(|models| models.onboarding.as_ref())
        {
            return onboarding::onboarding(model).into_any_element();
        }
        div()
            .id("shell")
            .size_full()
            .relative()
            .text_color(cx.theme().foreground)
            .text_size(px(12.0))
            // A drop the canvas slot did not take, on the toolbar, the
            // sidebar or the right panel, lands where a paste would.
            .on_drop(|paths: &ExternalPaths, _, _| {
                canvas::with(|canvas| {
                    (canvas.runtime).drop_files(paths.paths().iter().cloned(), None);
                });
            })
            .child(
                h_flex()
                    .size_full()
                    .items_start()
                    .child(
                        div()
                            .relative()
                            .size_full()
                            .child(self.canvas_slot(window, cx))
                            .children(
                                (models.as_ref())
                                    .map(|models| over_slot(models, window, cx))
                                    .unwrap_or_default(),
                            ),
                    )
                    .when_some(models.as_ref(), |row, models| {
                        row.children(self.chat_panel(&models.chat, window, cx))
                    }),
            )
            .when_some(models.as_ref(), |root, models: &Models| {
                root.when(models.sidebar.visible, |root| {
                    root.child(Self::sidebar(&models.sidebar, window, cx))
                })
                .child(
                    v_flex()
                        .id("chrome")
                        .occlude()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bg(theme::solid(theme::toolbar()))
                        .child(tabs::tabs(&models.strip, &models.toolbar)),
                )
                .when(models.toolbar.tools == ToolsPlace::Docked, |root| {
                    let left = covered_left(models) - TOOLS_DOCK_WIDTH;
                    let shown = models.tool_options.as_ref();
                    root.child(toolbar::docked(&models.toolbar, shown, left, window, cx))
                })
                .children(self.chat_resize_handle(&models.chat))
            })
            .child(
                // The hairline under the chrome is its own, drawn last so
                // nothing covers it.
                div()
                    .absolute()
                    .top(px(theme::CHROME_HEIGHT - 1.0))
                    .left_0()
                    .right_0()
                    .h(px(1.0))
                    .bg(theme::solid(theme::toolbar_border())),
            )
            .into_any_element()
    }
}
