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
//! The root view lays these out and draws none of them. The toolbar, the
//! sidebar, the tool's popup and the right panel are views of their own,
//! each holding its part of the models and cached: a frame builds the views
//! whose part changed, and the root, which is a handful of boxes. Nothing
//! here reads the models as it renders; [`ShellView::sync`] hands each view
//! its part when the canvas says one changed.
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
mod resize;
mod sidebar;
mod slot;
mod toolbar;

use std::path::PathBuf;
use std::rc::Rc;

use glam::Vec2;
use std::cell::Cell;
use std::path::Path;

use gpui_kit::component::{ActiveTheme as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, AppContext as _, Context, Entity, ExternalPaths, FocusHandle, Global,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, StyleRefinement, Styled as _,
    Window, div, px,
};
use specular_interact::{Action, CanvasAction, Event, OnboardingModel, SidebarAction};

use self::chat::ChatView;
pub(crate) use self::named::shown as shown_controls;
use self::popup::PopupView;
use self::sidebar::SidebarView;
use self::slot::Pointer;
use self::toolbar::ToolbarView;
use crate::canvas;
use crate::models::{Models, same};
use crate::surface::WindowAsks;
use crate::theme;
use crate::ui_stats::{self, Part};

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

/// What the root view lays out: which of the views are shown, and how wide
/// the right panel is.
#[derive(Debug, Clone, PartialEq, Default)]
struct Layout {
    sidebar: bool,
    popup: bool,
    /// The right panel's width, while it is open.
    chat: Option<f32>,
    /// The first-run view, shown in place of everything else.
    onboarding: Option<Rc<OnboardingModel>>,
}

impl Layout {
    fn of(models: &Models) -> Self {
        Self {
            sidebar: models.sidebar.visible,
            popup: models.popup.is_some(),
            chat: models.chat.visible.then_some(models.chat.width),
            onboarding: models.onboarding.clone(),
        }
    }

    fn same_as(&self, other: &Self) -> bool {
        (self.sidebar, self.popup, self.chat) == (other.sidebar, other.popup, other.chat)
            && same(self.onboarding.as_ref(), other.onboarding.as_ref())
    }
}

/// The root view: it holds what only a view can (focus, the input method's
/// marked text) and lays out the views that draw everything else.
pub(crate) struct ShellView {
    focus: FocusHandle,
    asks: Rc<WindowAsks>,
    pointer: Rc<Pointer>,
    /// The input method's marked text, which GPUI asks back for.
    marked: String,
    layout: Layout,
    toolbar: Entity<ToolbarView>,
    sidebar: Entity<SidebarView>,
    popup: Entity<PopupView>,
    chat: Entity<ChatView>,
    /// Whether the right panel's edge is being dragged.
    chat_resizing: Rc<Cell<bool>>,
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
            layout: Layout::default(),
            toolbar: cx.new(|_| ToolbarView::new()),
            sidebar: cx.new(|_| SidebarView::new()),
            popup: cx.new(|_| PopupView::new()),
            chat: cx.new(|cx| ChatView::new(window, cx)),
            chat_resizing: Rc::new(Cell::new(false)),
        }
    }

    /// Brings the views in step with the models: each is handed its part
    /// and draws again only if that is not the part it holds. The root
    /// draws again when what it lays out changed, or an effect asked
    /// something of the window that the slot applies.
    pub(crate) fn sync(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(models) = canvas::models_as_read() else {
            return;
        };
        let title = self.asks.title.borrow().clone();
        (self.toolbar).update(cx, |view, cx| view.set(&models.toolbar, &title, cx));
        (self.sidebar).update(cx, |view, cx| view.set(&models.sidebar, cx));
        (self.popup).update(cx, |view, cx| view.set(models.popup.as_ref(), cx));
        (self.chat).update(cx, |view, cx| view.set(&models.chat, window, cx));
        // A hidden field must not keep the keys.
        if !models.chat.visible && self.composer_focused(window, cx) {
            focus_canvas(window, cx);
        }
        let layout = Layout::of(&models);
        if !layout.same_as(&self.layout) || self.asks.changed.get() {
            self.layout = layout;
            cx.notify();
        }
    }

    /// Draws the views that show a text field again: one was opened in
    /// place of a label, or closed.
    pub(crate) fn fields_changed(&self, cx: &mut App) {
        (self.sidebar).update(cx, |_, cx| cx.notify());
        (self.popup).update(cx, |_, cx| cx.notify());
    }

    /// Whether the keys are in the right panel's field.
    pub(super) fn composer_focused(&self, window: &Window, cx: &App) -> bool {
        self.chat.read(cx).composer_focused(window, cx)
    }

    /// Focuses the right panel's field.
    pub(crate) fn focus_composer(&self, window: &mut Window, cx: &mut App) {
        let chat = self.chat.read(cx).field();
        chat.update(cx, |state, cx| state.focus(window, cx));
    }

    /// Pastes the image file at `path` into the right panel's composer as
    /// if it were on the clipboard, for the scripted-input driver.
    pub(crate) fn paste_image_file(&self, path: &Path, cx: &mut App) -> bool {
        let composer = self.chat.read(cx).composer();
        composer.update(cx, |view, cx| view.paste_image_file(path, cx))
    }

    /// Whether the keys are in one of the Kit's text fields: the composer,
    /// or a model field being typed in (a canvas's name, a popup's value).
    pub(crate) fn typing(&self, window: &Window, cx: &mut App) -> bool {
        let in_field = window
            .focused(cx)
            .is_some_and(|focused| field::holds_focus(&focused, window, cx));
        in_field || self.composer_focused(window, cx)
    }
}

/// `view` cached, filling the box the root gives it.
fn filling<V: Render>(view: &Entity<V>) -> impl IntoElement + use<V> {
    view.clone().cached(StyleRefinement::default().size_full())
}

impl Render for ShellView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        ui_stats::built(Part::Root);
        if ui_stats::gpui_frame_began() {
            // The update that draws this frame also presents it, and what
            // is deferred here runs when that update is over.
            cx.defer(|_| ui_stats::gpui_frame_ended());
        }
        // What effects asked of the window is applied by this render.
        self.asks.changed.set(false);
        let layout = self.layout.clone();
        if !layout.sidebar {
            named::forget(Part::Sidebar);
        }
        if !layout.popup {
            named::forget(Part::Popup);
        }
        if let Some(model) = layout.onboarding.as_deref() {
            named::forget(Part::Toolbar);
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
                    .when_some(layout.chat, |row, width| {
                        row.child(
                            div()
                                .w(px(width))
                                .h_full()
                                .flex_shrink_0()
                                .child(filling(&self.chat)),
                        )
                    }),
            )
            .when(layout.sidebar, |root| {
                root.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(theme::TOOLBAR_HEIGHT))
                        .bottom_0()
                        .w(px(theme::SIDEBAR_WIDTH))
                        .child(filling(&self.sidebar)),
                )
            })
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(theme::TOOLBAR_HEIGHT))
                    .child(filling(&self.toolbar)),
            )
            .when(layout.popup, |root| {
                // Everything under the toolbar, so the popup is as tall as
                // its controls make it. The box takes no pointer events.
                root.child(
                    div()
                        .absolute()
                        .top(px(theme::TOOLBAR_HEIGHT))
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .child(filling(&self.popup)),
                )
            })
            .children(layout.chat.map(|width| self.chat_resize_handle(width)))
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
