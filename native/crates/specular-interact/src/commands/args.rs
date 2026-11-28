//! Reading a command's JSON arguments, and the `enabled` rules the table
//! shares.

use serde_json::Value;
use specular_doc::{EntityId, ItemId};

use super::{Run, RunError};
use crate::menu::has_target;
use crate::{Action, App};

/// An action as what a command runs.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a command's build returns it as its own answer"
)]
pub(super) const fn act(action: Action) -> Result<Run, RunError> {
    Ok(Run::Action(action))
}

fn wrong(reason: impl Into<String>) -> RunError {
    RunError::Args(reason.into())
}

/// The text under `key`, when there is one.
pub(super) fn maybe_text(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// The text under `key`.
pub(super) fn text(args: &Value, key: &str) -> Result<String, RunError> {
    maybe_text(args, key).ok_or_else(|| wrong(format!("`{key}` is required")))
}

/// The entity `key` names, when it names one.
pub(super) fn maybe_entity(args: &Value, key: &str) -> Option<EntityId> {
    maybe_text(args, key).map(EntityId::from)
}

/// The entity `key` names.
pub(super) fn entity(args: &Value, key: &str) -> Result<EntityId, RunError> {
    text(args, key).map(EntityId::from)
}

/// The number under `key`.
pub(super) fn number(args: &Value, key: &str) -> Result<f64, RunError> {
    (args.get(key).and_then(Value::as_f64))
        .ok_or_else(|| wrong(format!("`{key}` is a number, and required")))
}

/// The yes or no under `key`.
pub(super) fn switch(args: &Value, key: &str) -> Result<bool, RunError> {
    (args.get(key).and_then(Value::as_bool))
        .ok_or_else(|| wrong(format!("`{key}` is true or false, and required")))
}

/// The texts of the list under `key`. None is an empty list.
pub(super) fn texts(args: &Value, key: &str) -> Result<Vec<String>, RunError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => (items.iter())
            .map(|item| item.as_str().map(str::to_owned))
            .collect::<Option<_>>()
            .ok_or_else(|| wrong(format!("`{key}` is a list of ids"))),
        Some(_) => Err(wrong(format!("`{key}` is a list of ids"))),
    }
}

/// The entities `ids` names, then the edges `edges` names.
pub(super) fn items(args: &Value) -> Result<Vec<ItemId>, RunError> {
    let entities = texts(args, "ids")?.into_iter();
    let edges = texts(args, "edges")?.into_iter();
    Ok((entities.map(|id| ItemId::Entity(id.into())))
        .chain(edges.map(|id| ItemId::Edge(id.into())))
        .collect())
}

/// The value of the word under `key`, from `words`.
pub(super) fn choice<T: Clone>(
    args: &Value,
    key: &str,
    words: &[(&str, T)],
) -> Result<T, RunError> {
    let named = text(args, key)?;
    (words.iter())
        .find(|(word, _)| *word == named)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| {
            let known: Vec<&str> = words.iter().map(|(word, _)| *word).collect();
            wrong(format!("`{key}` is one of {}", known.join(", ")))
        })
}

/// Always enabled.
pub(super) const fn always(_: &App) -> bool {
    true
}

/// Enabled while nothing is being dragged.
pub(super) fn idle(app: &App) -> bool {
    app.session.gesture.is_none()
}

/// Enabled while nothing is being dragged and `action` has something to
/// act on.
pub(super) fn can(app: &App, action: &Action) -> bool {
    idle(app) && has_target(app, action)
}
