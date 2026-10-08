//! The window's content: the Kit's toolbar and sidebar, and the slot the
//! canvas shows through.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────┐
//! │ toolbar (Kit), across the window                      │
//! ├──────────┬───────────────────────────────┬────────────┤
//! │ sidebar  │ canvas slot: nothing painted, │ right      │
//! │ (Kit)    │ so the canvas view under      │ panel      │
//! │          │ GPUI shows                    │ (Kit)      │
//! └──────────┴───────────────────────────────┴────────────┘
//! ```
//!
//! The slot fills the window beside the right panel, as the app's own
//! layout assumes. The toolbar and the sidebar lie over its top and left
//! edges: a popup beside a canvas item never rises above the toolbar strip,
//! and the app counts the sidebar's width as covered (`App::covered_left`)
//! while its model says it is visible, which is when the sidebar is drawn.
//! The right panel is beside the slot, so opening it or dragging its edge
//! makes the app's viewport narrower.

mod chat;
mod controls;
mod dropdown;
mod field;
mod glyphs;
mod ime;
mod menu;
mod named;
mod onboarding;
mod pick;
mod popup;
mod sidebar;
mod slot;
mod toolbar;

use std::path::PathBuf;
use std::rc::Rc;

use glam::Vec2;
use gpui_kit::component::{ActiveTheme as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Context, ExternalPaths, FocusHandle, Global, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, Window, div, px,
};
use specular_doc::ItemId;
use specular_interact::{Action, CanvasAction, Event, SidebarAction};

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
    /// or a model field being typed in (a canvas's name, a popup's value).
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
        named::begin_frame();
        let models = canvas::models();
        let title = self.asks.title.borrow().clone();
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
                    .child(self.canvas_slot(window, cx))
                    .when_some(models.as_ref(), |row, models| {
                        row.children(self.chat_panel(&models.chat, window, cx))
                    }),
            )
            .when_some(models.as_ref(), |root, models: &Models| {
                root.when(models.sidebar.visible, |root| {
                    root.child(Self::sidebar(&models.sidebar, window, cx))
                })
                .child(toolbar::toolbar(&models.toolbar, &title, cx))
                .when_some(models.popup.as_ref(), |root, model| {
                    root.child(popup::tool_popup(model, window, cx))
                })
                .children(self.chat_resize_handle(&models.chat))
            })
            .child(
                // The hairline under the toolbar is its own, drawn last so
                // nothing covers it.
                div()
                    .absolute()
                    .top(px(theme::TOOLBAR_HEIGHT - 1.0))
                    .left_0()
                    .right_0()
                    .h(px(1.0))
                    .bg(theme::solid(theme::TOOLBAR_BORDER)),
            )
            .into_any_element()
    }
}
