//! A mod's own line of text (ADR 0059): `Field({ id, placeholder, value,
//! autofocus })`, a field the window holds for it.
//!
//! The window keeps the input, so the keys stay in it while a person types
//! and Return reaches the mod: what was typed is the shared value
//! `field/<id>`, and each Return moves `field/<id>/submit` on by one, which
//! a mod reads with `shared(...)` and renders again on. A field is made
//! once for its id: a mod that wants a fresh one each time it opens (a
//! palette) names it anew, and the oldest of many is let go.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::component::Sizable as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::{
    AnyElement, App, AppContext as _, Entity, Focusable as _, IntoElement as _, SharedString,
    Styled as _, Subscription, Window, px,
};
use serde_json::Value;
use specular_mods::{HostValue, Revisions};

/// How many fields are kept before the oldest is let go.
const KEPT: usize = 32;

/// A field made for a mod, and when.
struct Held {
    input: Entity<InputState>,
    made: u64,
    _events: Subscription,
}

thread_local! {
    static HELD: RefCell<HashMap<String, Held>> = RefCell::new(HashMap::new());
    static MADE: Cell<u64> = const { Cell::new(0) };
}

/// Lets the oldest fields go while there are more than [`KEPT`].
fn trim(held: &mut HashMap<String, Held>) {
    while held.len() > KEPT {
        let oldest = (held.iter())
            .min_by_key(|(_, field)| field.made)
            .map(|(id, _)| id.clone());
        match oldest {
            Some(id) => held.remove(&id),
            None => return,
        };
    }
}

fn publish(revisions: &Rc<RefCell<Revisions>>, key: &str, value: Value, cx: &mut App) {
    if revisions.borrow_mut().publish(key, value) {
        cx.defer(App::refresh_windows);
    }
}

/// The field `props` asks for, made on first use.
pub(crate) fn field(
    props: &HostValue,
    style: crate::view::FieldStyle,
    revisions: &Rc<RefCell<Revisions>>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let text = |key: &str| {
        props
            .get(key)
            .and_then(HostValue::as_str)
            .map(str::to_owned)
    };
    let id = text("id").unwrap_or_else(|| "field".to_owned());
    let kept = HELD.with(|held| held.borrow().get(&id).map(|field| field.input.clone()));
    let input = kept.unwrap_or_else(|| {
        let value = text("value").unwrap_or_default();
        let hint = text("placeholder");
        let input = cx.new(|cx| {
            let state =
                InputState::new(window, cx).default_value(SharedString::from(value.clone()));
            match hint {
                Some(hint) => state.placeholder(SharedString::from(hint)),
                None => state,
            }
        });
        let key = format!("shared/field/{id}");
        publish(revisions, &key, Value::String(value), cx);
        let revisions = revisions.clone();
        let submits = Rc::new(Cell::new(0_u64));
        let events = cx.subscribe(&input, move |input, event: &InputEvent, cx| match event {
            InputEvent::Change => {
                let typed = input.read(cx).value().to_string();
                publish(&revisions, &key, Value::String(typed), cx);
            }
            InputEvent::PressEnter { .. } => {
                submits.set(submits.get() + 1);
                let number = Value::from(submits.get());
                publish(&revisions, &format!("{key}/submit"), number, cx);
            }
            InputEvent::Blur | InputEvent::Focus => {}
        });
        if props.get("autofocus").and_then(HostValue::as_bool) == Some(true) {
            input.update(cx, |state, cx| state.focus(window, cx));
        }
        let made = MADE.with(|made| {
            made.set(made.get() + 1);
            made.get()
        });
        HELD.with(|held| {
            let mut held = held.borrow_mut();
            let field = Held {
                input: input.clone(),
                made,
                _events: events,
            };
            held.insert(id, field);
            trim(&mut held);
        });
        input
    });
    let focused = input.read(cx).focus_handle(cx).is_focused(window);
    style
        .over(Input::new(&input).small().text_size(px(13.0)), focused)
        .into_any_element()
}
