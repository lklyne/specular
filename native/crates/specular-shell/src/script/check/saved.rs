//! Facts on the canvas as a save would write it.

use anyhow::{Context as _, bail, ensure};
use serde_json::Value;
use specular_interact::{App, CanvasId};

use super::super::expect::SavedFact;
use super::super::saved::{at, same};
use super::resolve_path;
use crate::persist::canvas_text;

/// Whether `fact` holds at `path` in the canvas as a save would write it.
pub(super) fn saved_fact(app: &App, path: &str, fact: &SavedFact) -> anyhow::Result<()> {
    let canvas = saved_json(app)?;
    let path = resolve_path(app, path)?;
    let found = at(&canvas, &path);
    let shown = || found.map_or("nothing".to_owned(), Value::to_string);
    match fact {
        SavedFact::Equals(value) => ensure!(
            found.is_some_and(|found| same(found, value)),
            "saved {path} is {}, expected {value}",
            shown()
        ),
        SavedFact::Absent => ensure!(found.is_none(), "saved {path} is {}", shown()),
        SavedFact::Keys(keys) => {
            let Some(Value::Object(map)) = found else {
                bail!("saved {path} is {}, not an object", shown());
            };
            let mut have: Vec<&str> = map.keys().map(String::as_str).collect();
            let mut want: Vec<&str> = keys.iter().map(String::as_str).collect();
            have.sort_unstable();
            want.sort_unstable();
            ensure!(have == want, "saved {path} has keys {}", have.join(" "));
        }
        SavedFact::Length(length) => {
            let have = match found {
                Some(Value::Array(items)) => items.len(),
                Some(Value::Object(map)) => map.len(),
                _ => bail!("saved {path} is {}, not a list", shown()),
            };
            ensure!(have == *length, "saved {path} has {have} items");
        }
    }
    Ok(())
}

/// The active canvas as a save would write it now.
pub(crate) fn saved_text(app: &App) -> anyhow::Result<String> {
    saved_text_of(app, &app.space().active().id)
}

fn saved_text_of(app: &App, id: &CanvasId) -> anyhow::Result<String> {
    let canvas = app
        .canvas_to_save(id)
        .context("the canvas showing is not in the space")?;
    canvas_text(&canvas).context("writing the canvas as text")
}

pub(super) fn saved_json(app: &App) -> anyhow::Result<Value> {
    saved_json_of(app, &app.space().active().id)
}

pub(super) fn saved_json_of(app: &App, id: &CanvasId) -> anyhow::Result<Value> {
    Ok(serde_json::from_str(&saved_text_of(app, id)?)?)
}
