//! The open thread's conversation: `ThreadTranscript` in `ChatPane.tsx` and
//! `CommentBubble` in `CommentPrimitives.tsx`.
//!
//! The rows are the sent messages, then what the agent has said so far this
//! run, then the last run's error or the empty hint. They sit in the Kit's
//! `MessageScroller`, which follows the end as rows arrive and grow unless
//! the reader has scrolled up. The run bar is not a row: it is a view of
//! its own under the conversation, so its shimmer does not lay these rows
//! out again every frame.

use std::hash::{Hash as _, Hasher as _};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::bubble::{Bubble as KitBubble, BubbleContent, BubbleVariant};
use gpui_kit::component::h_flex;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::message::MessageAlignment;
use gpui_kit::component::message_scroller::{MessageScroller, MessageScrollerState};
use gpui_kit::component::text::TextView;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, Context, Entity, Hsla, InteractiveElement as _,
    IntoElement, ObjectFit, ParentElement as _, Render, Rgba, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled as _, StyledImage as _, Window, div,
    img, px, relative, rgb,
};
use specular_interact::{Bubble, ThreadId, ThreadRole};

use super::header::thread_list;
use super::parts::{BodyPart, Conversation};
use crate::models::same;
use crate::theme;
use crate::ui_stats::{self, Part};
use crate::view::run;

/// Where a sent message sits. `CommentBubble` lays the person's bubble out
/// inline at the leading edge, like the agent's words under it; only the
/// bubble tells the two apart.
const USER_SIDE: MessageAlignment = MessageAlignment::Start;
/// How long a bubble stays darker after its comment is focused, `FLASH_MS`.
const FLASH: Duration = Duration::from_millis(200);
/// How far the flash moves the bubble's fill toward its text colour.
const FLASH_MIX: f32 = 0.12;

/// One row of the conversation.
#[derive(Debug, Clone, PartialEq)]
enum Row {
    Message(Bubble),
    Streaming(String),
    Error(String),
    Hint(String),
}

fn rows_of(conversation: &Conversation) -> Vec<Row> {
    let mut rows: Vec<Row> = (conversation.messages.iter().cloned())
        .map(Row::Message)
        .collect();
    rows.extend(conversation.streaming.clone().map(Row::Streaming));
    rows.extend(conversation.error.clone().map(Row::Error));
    rows.extend(conversation.empty_hint.clone().map(Row::Hint));
    rows
}

/// What a row's height depends on, so a row that changed is measured again.
fn signature(row: &Row) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match row {
        Row::Message(bubble) => {
            (0, &bubble.id, &bubble.text, bubble.images.len()).hash(&mut hasher);
        }
        Row::Streaming(text) => (1, text).hash(&mut hasher),
        Row::Error(text) => (3, text).hash(&mut hasher),
        Row::Hint(text) => (4, text).hash(&mut hasher),
    }
    hasher.finish()
}

/// What the scroller was last told it shows.
#[derive(Debug, Default)]
struct Shown {
    thread: Option<ThreadId>,
    rows: Vec<u64>,
    /// The bubble of the focused comment.
    focused: Option<String>,
    /// The bubble that flashes, and how many flashes there have been: a new
    /// number starts the animation again.
    flash: Option<(String, u32)>,
}

fn mix(from: u32, to: u32, amount: f32) -> Hsla {
    let (from, to) = (rgb(from), rgb(to));
    let lerp = |from: f32, to: f32| from + (to - from) * amount;
    Rgba {
        r: lerp(from.r, to.r),
        g: lerp(from.g, to.g),
        b: lerp(from.b, to.b),
        a: 1.0,
    }
    .into()
}

/// The surface of a sent message: the composer's own fill and edge, so it
/// looks like the field it came from.
fn surface() -> BubbleContent {
    BubbleContent::new()
        .rounded(px(16.0))
        .border_color(theme::solid(theme::INPUT_BORDER))
        .bg(theme::solid(theme::INPUT))
        .text_color(theme::solid(theme::TEXT))
        .px_3()
        .py(px(6.0))
        .text_size(px(12.0))
        .line_height(relative(1.625))
}

fn thumbnails(images: &[String]) -> impl IntoElement + use<> {
    h_flex()
        .flex_wrap()
        .gap(px(6.0))
        .py(px(6.0))
        .children(images.iter().map(|path| {
            img(PathBuf::from(path))
                .size(px(48.0))
                .flex_shrink_0()
                .rounded(px(6.0))
                .border_1()
                .border_color(theme::solid(theme::ZINC_300))
                .object_fit(ObjectFit::Cover)
        }))
}

/// The person's message: a bubble, shown exactly as typed.
fn user_bubble(bubble: &Bubble, flash: Option<u32>) -> AnyElement {
    let body = KitBubble::new()
        .with_variant(BubbleVariant::Outline)
        .alignment(USER_SIDE)
        .max_w_full()
        .content(surface())
        .when(!bubble.images.is_empty(), |this| {
            this.child(thumbnails(&bubble.images))
        })
        .when(!bubble.text.is_empty(), |this| {
            this.child(SharedString::from(bubble.text.clone()))
        });
    let body = match flash {
        Some(count) => body
            .with_animation(
                SharedString::from(format!("chat-flash-{}-{count}", bubble.id)),
                Animation::new(FLASH).with_easing(gpui_kit::ease_out_quint()),
                |body, progress| {
                    let fill = mix(theme::INPUT, theme::TEXT, FLASH_MIX * (1.0 - progress));
                    body.content(surface().bg(fill))
                },
            )
            .into_any_element(),
        None => body.into_any_element(),
    };
    let Some(focus) = bubble.focus.clone() else {
        return body;
    };
    // The comment's pin is still on the canvas: a click brings it into view.
    div()
        .id(SharedString::from(format!("chat-bubble-{}", bubble.id)))
        .flex()
        .cursor_pointer()
        .on_click(move |_, window, cx| run(&focus, window, cx))
        .child(body)
        .into_any_element()
}

/// The agent's words: markdown, with no bubble around it.
fn agent_text(id: SharedString, text: &str) -> AnyElement {
    KitBubble::new()
        .with_variant(BubbleVariant::Ghost)
        .alignment(MessageAlignment::Start)
        .child(
            TextView::markdown(id, SharedString::from(text.to_owned()))
                .selectable(true)
                .text_size(px(12.0))
                .line_height(relative(1.625)),
        )
        .into_any_element()
}

fn note(text: &str, color: Hsla) -> AnyElement {
    div()
        .text_size(px(12.0))
        .text_color(color)
        .child(SharedString::from(text.to_owned()))
        .into_any_element()
}

/// The panel's body: the canvas's threads, or the open one's conversation.
pub(super) struct BodyView {
    part: Option<Rc<BodyPart>>,
    /// The conversation's rows, as the part has them.
    rows: Rc<Vec<Row>>,
    /// The composer's field, which a thread chosen from the list gives the
    /// keys to.
    input: Entity<TextareaState>,
    /// Where the conversation is scrolled, and whether it follows its end.
    scroller: Entity<MessageScrollerState>,
    shown: Shown,
}

impl BodyView {
    pub(super) fn new(
        input: Entity<TextareaState>,
        scroller: Entity<MessageScrollerState>,
    ) -> Self {
        Self {
            part: None,
            rows: Rc::default(),
            input,
            scroller,
            shown: Shown::default(),
        }
    }

    /// Shows `part`, and draws again if it is not the one shown.
    pub(super) fn set(&mut self, part: &Rc<BodyPart>, cx: &mut Context<'_, Self>) {
        if same(self.part.as_ref(), Some(part)) {
            return;
        }
        self.part = Some(Rc::clone(part));
        if let BodyPart::Conversation(conversation) = &**part {
            let rows = rows_of(conversation);
            self.sync_scroller(conversation.thread.clone(), &rows, cx);
            self.rows = Rc::new(rows);
        }
        cx.notify();
    }

    /// Brings the scroller in step with `rows`: a new thread starts at its
    /// end, new rows are appended, and a row whose words changed is measured
    /// again. Also notes the comment that was just focused, to flash its
    /// bubble and bring it into view.
    fn sync_scroller(
        &mut self,
        thread: Option<ThreadId>,
        rows: &[Row],
        cx: &mut Context<'_, Self>,
    ) {
        let signatures: Vec<u64> = rows.iter().map(signature).collect();
        let shown = &mut self.shown;
        let (old, new) = (shown.rows.len(), signatures.len());
        if thread != shown.thread {
            self.scroller.update(cx, |state, cx| state.reset(new, cx));
        } else if signatures != shown.rows {
            let changed: Vec<usize> = (0..old.min(new))
                .filter(|&index| signatures.get(index) != shown.rows.get(index))
                .collect();
            self.scroller.update(cx, |state, cx| {
                if new >= old {
                    state.append(new - old, cx);
                } else {
                    state.splice(new..old, 0, cx);
                }
                for index in changed {
                    state.remeasure_items(index..index + 1, cx);
                }
            });
        }
        shown.thread = thread;
        shown.rows = signatures;

        let focused = rows.iter().enumerate().find_map(|(index, row)| match row {
            Row::Message(bubble) if bubble.focused => Some((index, bubble.id.clone())),
            Row::Message(_) | Row::Streaming(_) | Row::Error(_) | Row::Hint(_) => None,
        });
        let id = focused.as_ref().map(|(_, id)| id.clone());
        if id != shown.focused {
            shown.focused = id;
            if let Some((index, id)) = focused {
                let count = shown.flash.as_ref().map_or(0, |(_, count)| count + 1);
                shown.flash = Some((id, count));
                self.scroller.update(cx, |state, cx| {
                    if state.is_scrolled_up() {
                        state.scroll_to_item(index, cx);
                    }
                });
            }
        }
    }

    /// The open thread's conversation.
    fn conversation(&self) -> impl IntoElement + use<> {
        let rows = Rc::clone(&self.rows);
        let flash = self.shown.flash.clone();
        MessageScroller::new(
            "chat-transcript",
            self.scroller.clone(),
            move |index, _, _| match rows.get(index) {
                Some(Row::Message(bubble)) => match bubble.role {
                    ThreadRole::User => {
                        let flash = (flash.as_ref())
                            .filter(|(id, _)| *id == bubble.id)
                            .map(|(_, count)| *count);
                        user_bubble(bubble, flash)
                    }
                    ThreadRole::Agent => agent_text(
                        SharedString::from(format!("chat-agent-{}", bubble.id)),
                        &bubble.text,
                    ),
                },
                Some(Row::Streaming(text)) => agent_text("chat-streaming".into(), text),
                Some(Row::Error(text)) => note(text, theme::solid(theme::ERROR)),
                Some(Row::Hint(text)) => note(text, theme::tinted(theme::TEXT_MUTED)),
                None => div().into_any_element(),
            },
        )
        // `space-y-4` between rows and `py-2.5` around them.
        .with_row_style(StyleRefinement::default().pb_4())
        .with_list_style(StyleRefinement::default().pt(px(10.0)).pb_0())
        .jump_button(false)
    }
}

impl Render for BodyView {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        ui_stats::built(Part::Transcript);
        match self.part.as_deref() {
            Some(BodyPart::Threads(threads)) => {
                thread_list(threads, &self.input).into_any_element()
            }
            Some(BodyPart::Conversation(_)) => div()
                .size_full()
                .child(self.conversation())
                .into_any_element(),
            None => div().into_any_element(),
        }
    }
}
