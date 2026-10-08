//! A model [`Field`] as a Kit text field.
//!
//! The model says what the field holds and what its typed text means. The
//! Kit's input keeps the caret, the selection and the focus while a person
//! types. Return and a click elsewhere send what the text asks for, as
//! [`FieldSubmit::action`](specular_interact::FieldSubmit) has it; Escape
//! puts the old value back. Either way the keys go back to the canvas.

use std::collections::HashMap;

use gpui_kit::component::input::{Escape, Input, InputEvent, InputState};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Sizable as _, h_flex};
use gpui_kit::{
    AnyElement, App, AppContext as _, Entity, FocusHandle, Focusable as _, Global,
    InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
};
use specular_interact::{ControlId, Event, Field, FieldWidth};

use super::focus_canvas;
use super::named::mark;
use crate::{canvas, shell};

/// A field being shown: the Kit's input, and the model it was made from.
struct Held {
    input: Entity<InputState>,
    model: Field,
    _events: Subscription,
}

/// The fields on screen, by name, and the one opened in place of a label.
#[derive(Default)]
struct Fields {
    held: HashMap<ControlId, Held>,
    inline: Option<ControlId>,
}

impl Global for Fields {}

/// How a field's editing ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    /// Return: send the text and give the keys back.
    Enter,
    /// The focus went elsewhere: send the text and leave the focus there.
    Blur,
    /// Escape: drop the text and give the keys back.
    Cancel,
}

/// The width a field asks for, in pixels.
const fn width(width: FieldWidth) -> f32 {
    match width {
        FieldWidth::Wide => 220.0,
        FieldWidth::Medium => 120.0,
        FieldWidth::Short => 52.0,
    }
}

/// Ends the editing of the field `id`. The input is dropped, so the next
/// frame builds one from the model's value as it then is.
fn end(id: &ControlId, how: End, cx: &mut App) {
    let fields = cx.default_global::<Fields>();
    let Some(held) = fields.held.remove(id) else {
        return;
    };
    if fields.inline.as_ref() == Some(id) {
        fields.inline = None;
    }
    let text = held.input.read(cx).value().to_string();
    if how != End::Cancel
        && text != held.model.value
        && let Some(action) = held.model.submit.action(&text)
    {
        canvas::dispatch(Event::Action(action));
    }
    shell::with_view(cx, move |_, window, cx| {
        if how != End::Blur {
            focus_canvas(window, cx);
        }
        cx.notify();
    });
}

/// The Kit input of `model`, made on first use and again whenever the
/// model's value changed while nobody was typing in it.
fn input_of(model: &Field, window: &mut Window, cx: &mut App) -> Entity<InputState> {
    let held = cx.default_global::<Fields>().held.get(&model.id);
    let kept = held.map(|held| (held.input.clone(), held.model == *model));
    if let Some((input, same)) = kept
        && (same || input.read(cx).focus_handle(cx).is_focused(window))
    {
        return input;
    }
    let value = SharedString::from(model.value.clone());
    let hint = (model.placeholder.as_ref()).map(|text| SharedString::from(text.to_string()));
    let input = cx.new(|cx| {
        let state = InputState::new(window, cx).default_value(value);
        match hint {
            Some(hint) => state.placeholder(hint),
            None => state,
        }
    });
    let id = model.id.clone();
    let events = cx.subscribe(&input, move |_, event: &InputEvent, cx| match event {
        InputEvent::PressEnter { .. } => end(&id, End::Enter, cx),
        InputEvent::Blur => end(&id, End::Blur, cx),
        InputEvent::Change | InputEvent::Focus => {}
    });
    let held = Held {
        input: input.clone(),
        model: model.clone(),
        _events: events,
    };
    cx.default_global::<Fields>()
        .held
        .insert(model.id.clone(), held);
    input
}

/// The input of `model` alone, as wide as what it is put in.
pub(super) fn input(model: &Field, window: &mut Window, cx: &mut App) -> AnyElement {
    let input = input_of(model, window, cx);
    let (id, label) = (model.id.clone(), model.label.to_string());
    div()
        .id(SharedString::from(model.id.as_str().to_owned()))
        .w_full()
        // Escape drops what was typed before the input sees the key.
        .capture_action(move |_: &Escape, _, cx| end(&id, End::Cancel, cx))
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .child(Input::new(&input).xsmall())
        .child(mark(&model.id))
        .into_any_element()
}

/// `model` as a popup control: its caption, then its input at the width it
/// asks for.
pub(super) fn field(model: &Field, window: &mut Window, cx: &mut App) -> AnyElement {
    let caption = (model.caption.as_ref()).map(|text| SharedString::from(text.to_string()));
    h_flex()
        .gap_1()
        .items_center()
        .text_size(px(12.0))
        .children(caption)
        .child(
            div()
                .w(px(width(model.width)))
                .child(input(model, window, cx)),
        )
        .into_any_element()
}

/// Whether `focused` is the focus of a field being typed in.
pub(super) fn holds_focus(focused: &FocusHandle, window: &Window, cx: &mut App) -> bool {
    let inputs: Vec<Entity<InputState>> = (cx.default_global::<Fields>().held.values())
        .map(|held| held.input.clone())
        .collect();
    (inputs.iter()).any(|input| input.read(cx).focus_handle(cx).contains(focused, window))
}

/// Whether `id` is the field opened in place of its label.
pub(super) fn is_inline(id: &ControlId, cx: &mut App) -> bool {
    cx.default_global::<Fields>().inline.as_ref() == Some(id)
}

/// Opens `model` in place of its label, with its text selected.
pub(super) fn begin_inline(model: &Field, window: &mut Window, cx: &mut App) {
    let open = cx.default_global::<Fields>().inline.clone();
    if let Some(open) = open.filter(|open| *open != model.id) {
        end(&open, End::Blur, cx);
    }
    let input = input_of(model, window, cx);
    cx.default_global::<Fields>().inline = Some(model.id.clone());
    input.update(cx, |state, cx| {
        state.focus(window, cx);
        state.select_all(window, cx);
    });
}
