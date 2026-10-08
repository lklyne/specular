//! The right panel, from [`ChatModel`]: the canvas's agent threads, the
//! open thread's conversation, and the composer.
//!
//! ```text
//! ┌ header: title (the thread switcher), new, close, back ┐
//! ├ the thread list, or the open thread's conversation    ┤
//! ├ the run bar, while the agent works                    ┤
//! ├ composer: chips, pasted images, the field, send       ┘
//! ```
//!
//! Each of the four is a view of its own, holding its part of the model
//! (`parts`). The header and the body are cached, so a frame that only
//! moves the run bar's shimmer builds the run bar and nothing above it.
//! Every control sends the model's own `Action`. The views keep only what
//! a view must: the field's text, the images pasted into it, where the
//! conversation is scrolled, and whether the run log is open.

mod chips;
mod composer;
mod date;
mod header;
mod parts;
mod paste;
mod run_bar;
mod transcript;

use std::rc::Rc;

use gpui_kit::component::input::TextareaState;
use gpui_kit::component::message_scroller::MessageScrollerState;
use gpui_kit::component::v_flex;
use gpui_kit::{
    App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, StyleRefinement, Styled as _, Window, div, px,
};
use specular_interact::{Action, ChatModel, Event};

pub(super) use self::composer::ComposerView;
use self::header::HeaderView;
use self::parts::ChatParts;
use self::run_bar::RunBarView;
use self::transcript::BodyView;
use crate::canvas as app_canvas;
use crate::models::same;
use crate::theme;

/// The header strip's height.
const HEADER: f32 = 36.0;

/// Runs a panel control's `action` and leaves the keys in the composer's
/// field: the panel has one field, and what is clicked beside it (a thread,
/// New) is followed by typing.
fn run_keeping_field(
    action: &Action,
    input: &Entity<TextareaState>,
    window: &mut Window,
    cx: &mut App,
) {
    app_canvas::dispatch(Event::Action(action.clone()));
    input.update(cx, |state, cx| state.focus(window, cx));
}

/// The right panel: where its four views go.
pub(super) struct ChatView {
    /// The model the parts were last cut from.
    model: Option<Rc<ChatModel>>,
    parts: Option<ChatParts>,
    /// The composer's field, which the header's buttons give the keys to.
    input: Entity<TextareaState>,
    header: Entity<HeaderView>,
    body: Entity<BodyView>,
    run_bar: Entity<RunBarView>,
    composer: Entity<ComposerView>,
}

impl ChatView {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let input = composer::field(window, cx);
        let scroller = cx.new(|cx| MessageScrollerState::new(0, cx));
        let composer = {
            let (input, scroller) = (input.clone(), scroller.clone());
            cx.new(|cx| ComposerView::new(input, scroller, window, cx))
        };
        Self {
            model: None,
            parts: None,
            header: cx.new(|_| HeaderView::new(input.clone())),
            body: cx.new(|_| BodyView::new(input.clone(), scroller)),
            run_bar: cx.new(|_| RunBarView::new()),
            composer,
            input,
        }
    }

    /// Shows `model`: each view whose part changed is told to draw again.
    pub(super) fn set(
        &mut self,
        model: &Rc<ChatModel>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if same(self.model.as_ref(), Some(model)) {
            return;
        }
        self.model = Some(Rc::clone(model));
        let parts = match self.parts.as_mut() {
            Some(parts) => {
                parts.refresh(model);
                parts.clone()
            }
            None => self.parts.insert(ChatParts::of(model)).clone(),
        };
        // The run bar comes and goes, which moves the views around it.
        let running = self.run_bar.read(cx).is_running();
        if running != parts.run.is_some() {
            cx.notify();
        }
        (self.header).update(cx, |view, cx| view.set(&parts.header, cx));
        (self.body).update(cx, |view, cx| view.set(&parts.body, cx));
        (self.run_bar).update(cx, |view, cx| view.set(parts.run.as_ref(), cx));
        let visible = model.visible;
        (self.composer).update(cx, |view, cx| {
            view.set(&parts.composer, visible, window, cx);
        });
    }

    /// Whether the keys are in the composer's field.
    pub(super) fn composer_focused(&self, window: &Window, cx: &App) -> bool {
        self.input.focus_handle(cx).is_focused(window)
    }

    /// The composer's field.
    pub(super) fn field(&self) -> Entity<TextareaState> {
        self.input.clone()
    }

    /// The composer.
    pub(super) fn composer(&self) -> Entity<ComposerView> {
        self.composer.clone()
    }
}

impl Render for ChatView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let running = self.run_bar.read(cx).is_running();
        let header = StyleRefinement::default()
            .w_full()
            .h(px(HEADER))
            .flex_shrink_0();
        let body = StyleRefinement::default().w_full().flex_1().min_h_0();
        v_flex()
            .id("chat-panel")
            .occlude()
            .size_full()
            .pt(px(theme::TOOLBAR_HEIGHT))
            .bg(theme::solid(theme::PANEL))
            .border_l_1()
            .border_color(theme::solid(theme::CHROME_BORDER))
            .child(self.header.clone().cached(header))
            .child(self.body.clone().cached(body))
            // The bar is as tall as its log makes it and the composer as
            // its text does, so neither can be laid out without being built.
            .children(running.then(|| div().flex_shrink_0().child(self.run_bar.clone())))
            .child(div().flex_shrink_0().child(self.composer.clone()))
    }
}
