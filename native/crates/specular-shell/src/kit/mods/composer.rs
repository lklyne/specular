//! A mod's agent composer: `Composer({ id, thread, placeholder, autofocus })`,
//! the right panel's composer box for any thread.
//!
//! It is the panel's own box (`view::composer_box`): the pasted images, the
//! field, the context pill, the folder, the send mode, the model and Send.
//! Return or Send sends the words and the images to `thread`, which is
//! started under that id if it is not made yet, or to the thread the panel
//! has open when no thread is named. Shift+Return opens a line, and an
//! image pasted into the field waits above it.
//!
//! The window keeps each composer by its `id`, as a view of its own, so it
//! redraws as a person types or pastes without the mod rendering again.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Subscription, Window, div,
};
use specular_agent::ThreadId;
use specular_interact::{Action, ChatAction, Event};
use specular_mods::HostValue;

use crate::canvas;
use crate::view::composer_box::{self, ComposerBox, OnPaste, OnRemove, OnSend};
use crate::view::paste::{Pasted, images_of};

/// How many composers are kept before the oldest is let go.
const KEPT: usize = 16;

/// The hint an empty field shows when the mod gives none.
const HINT: &str = "Write to the agent…";

/// One composer's field, images and thread.
pub(crate) struct ComposerView {
    input: Entity<TextareaState>,
    images: Vec<Pasted>,
    thread: Option<ThreadId>,
    placeholder: String,
    _events: Subscription,
}

impl ComposerView {
    fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        // Enter sends and Shift+Enter opens a line: the field reports a
        // plain Enter and inserts nothing for it.
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(composer_box::MIN_ROWS, composer_box::MAX_ROWS)
                .submit_on_enter(true)
                .placeholder(SharedString::from(HINT))
        });
        let events =
            cx.subscribe_in(
                &input,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { shift: false, .. } => this.send(window, cx),
                    InputEvent::Change => cx.notify(),
                    InputEvent::PressEnter { .. } | InputEvent::Focus | InputEvent::Blur => {}
                },
            );
        Self {
            input,
            images: Vec::new(),
            thread: None,
            placeholder: HINT.to_owned(),
            _events: events,
        }
    }

    /// Sends the words and the images, and empties the field.
    fn send(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = self.input.read(cx).value().trim().to_owned();
        if text.is_empty() && self.images.is_empty() {
            return;
        }
        let images = (self.images.drain(..))
            .map(|pasted| pasted.upload)
            .collect();
        canvas::dispatch(Event::Action(Action::Chat(ChatAction::Send {
            text,
            images,
            thread: self.thread.clone(),
        })));
        self.input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    /// Takes the props the mod drew it with.
    fn take(&mut self, props: &HostValue, window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = |key: &str| props.get(key).and_then(HostValue::as_str);
        self.thread = text("thread")
            .filter(|id| !id.is_empty())
            .map(|id| ThreadId(id.to_owned()));
        let placeholder = text("placeholder").unwrap_or(HINT);
        if self.placeholder != placeholder {
            placeholder.clone_into(&mut self.placeholder);
            let hint = SharedString::from(placeholder.to_owned());
            self.input
                .update(cx, |state, cx| state.set_placeholder(hint, window, cx));
        }
    }
}

impl Render for ComposerView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let Some(models) = canvas::models() else {
            return div().into_any_element();
        };
        let model = &models.chat.composer;
        let typed = !self.input.read(cx).value().trim().is_empty();
        let view = cx.entity().downgrade();
        let on_paste: OnPaste = {
            let view = view.clone();
            Rc::new(move |item, _, cx| {
                let images = images_of(item);
                if images.is_empty() {
                    return false;
                }
                let _ = view.update(cx, |this, cx| {
                    this.images.extend(images);
                    cx.notify();
                });
                true
            })
        };
        let on_send: OnSend = {
            let view = view.clone();
            Rc::new(move |window, cx| {
                let _ = view.update(cx, |this, cx| this.send(window, cx));
            })
        };
        let on_remove: OnRemove = Rc::new(move |index, _, cx| {
            let _ = view.update(cx, |this, cx| {
                if index < this.images.len() {
                    this.images.remove(index);
                    cx.notify();
                }
            });
        });
        let parts = ComposerBox {
            model,
            input: &self.input,
            images: &self.images,
            ready: typed || !self.images.is_empty(),
        };
        composer_box::frame("composer")
            .children(parts.thumbnails(&on_remove))
            .child(parts.field(on_paste))
            .child(parts.chips(on_send))
            .into_any_element()
    }
}

/// A composer made for a mod, and when.
struct Held {
    view: Entity<ComposerView>,
    made: u64,
}

thread_local! {
    static HELD: RefCell<HashMap<String, Held>> = RefCell::new(HashMap::new());
    static MADE: Cell<u64> = const { Cell::new(0) };
}

/// The composer `props` asks for, made on first use.
pub(crate) fn composer(props: &HostValue, window: &mut Window, cx: &mut App) -> AnyElement {
    let id = (props.get("id").and_then(HostValue::as_str))
        .unwrap_or("composer")
        .to_owned();
    let kept = HELD.with(|held| held.borrow().get(&id).map(|held| held.view.clone()));
    let fresh = kept.is_none();
    let view = kept.unwrap_or_else(|| {
        let view = cx.new(|cx| ComposerView::new(window, cx));
        let made = MADE.with(|made| {
            made.set(made.get() + 1);
            made.get()
        });
        HELD.with(|held| {
            let mut held = held.borrow_mut();
            held.insert(
                id.clone(),
                Held {
                    view: view.clone(),
                    made,
                },
            );
            while held.len() > KEPT {
                let oldest = (held.iter())
                    .min_by_key(|(_, each)| each.made)
                    .map(|(id, _)| id.clone());
                let Some(oldest) = oldest else { break };
                held.remove(&oldest);
            }
        });
        view
    });
    view.update(cx, |this, cx| {
        this.take(props, window, cx);
        if fresh && props.get("autofocus").and_then(HostValue::as_bool) == Some(true) {
            this.input.update(cx, |state, cx| state.focus(window, cx));
        }
    });
    div()
        .id(SharedString::from(format!("composer.{id}")))
        .w_full()
        .child(view)
        .into_any_element()
}
