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
//! The slot runs up under the toolbar, as the app's own layout assumes: a
//! popup beside a canvas item never rises above the toolbar strip. The
//! right panel is beside the slot, so opening it or dragging its edge
//! makes the app's viewport narrower.

mod chat;
mod controls;
mod glyphs;
mod ime;
mod onboarding;
mod popup;
mod sidebar;
mod slot;
mod toolbar;

use std::rc::Rc;

use gpui_kit::component::input::InputState;
use gpui_kit::component::{ActiveTheme as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Context, Entity, FocusHandle, Global, IntoElement, ParentElement as _, Render,
    Styled as _, Subscription, Window, div, px,
};
use specular_interact::{Action, CanvasId, Event};

use self::slot::Pointer;
use crate::canvas::{self, Models};
use crate::surface::WindowAsks;
use crate::theme;

/// The canvas slot's focus, so a click on a Kit control can hand the keys
/// back to the canvas.
pub(crate) struct CanvasFocus(pub(crate) FocusHandle);

impl Global for CanvasFocus {}

/// Runs `action` through `update` and gives the keys back to the canvas.
pub(crate) fn run(action: &Action, window: &mut Window, cx: &mut App) {
    canvas::dispatch(Event::Action(action.clone()));
    focus_canvas(window, cx);
}

/// Gives the keys back to the canvas.
pub(crate) fn focus_canvas(window: &mut Window, cx: &mut App) {
    if let Some(focus) = cx.try_global::<CanvasFocus>().map(|focus| focus.0.clone()) {
        window.focus(&focus, cx);
    }
}

/// A canvas being renamed in place in the sidebar.
struct Rename {
    canvas: CanvasId,
    input: Entity<InputState>,
    _events: Subscription,
}

/// The root view: it holds what only a view can (focus, the text field of
/// a rename, the input method's marked text) and draws everything else
/// from the models.
pub(crate) struct ShellView {
    focus: FocusHandle,
    asks: Rc<WindowAsks>,
    pointer: Rc<Pointer>,
    /// The input method's marked text, which GPUI asks back for.
    marked: String,
    rename: Option<Rename>,
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
            rename: None,
            chat: chat::ChatUi::new(window, cx),
        }
    }
}

impl ShellView {
    /// Whether the keys are in one of the Kit's text fields: the composer,
    /// or a canvas's name being typed.
    pub(crate) fn typing(&self, window: &Window, cx: &App) -> bool {
        use gpui_kit::Focusable as _;
        let renaming = (self.rename.as_ref())
            .is_some_and(|rename| rename.input.focus_handle(cx).is_focused(window));
        renaming || self.composer_focused(window, cx)
    }
}

impl Render for ShellView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        // What effects asked of the window is applied by this render.
        self.asks.changed.set(false);
        let models = canvas::models();
        let title = self.asks.title.borrow().clone();
        if let Some(model) = models
            .as_ref()
            .and_then(|models| models.onboarding.as_ref())
        {
            return onboarding::onboarding(model).into_any_element();
        }
        div()
            .size_full()
            .relative()
            .text_color(cx.theme().foreground)
            .text_size(px(12.0))
            .child(
                h_flex()
                    .size_full()
                    .items_start()
                    .when_some(models.as_ref(), |row, models| {
                        row.child(self.sidebar(&models.sidebar, window, cx))
                    })
                    .child(self.canvas_slot(window, cx))
                    .when_some(models.as_ref(), |row, models| {
                        row.children(self.chat_panel(&models.chat, window, cx))
                    }),
            )
            .when_some(models.as_ref(), |root, models: &Models| {
                root.child(toolbar::toolbar(&models.toolbar, &title, cx))
                    .when_some(models.popup.as_ref(), |root, model| {
                        root.child(popup::tool_popup(model, cx))
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
