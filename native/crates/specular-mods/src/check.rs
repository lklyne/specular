//! `specular mod check`: a mod loaded against a state fixture and rendered
//! once with no app behind it, so an author (or their agent) sees the
//! element tree it describes, with ids, before it ever reaches a window.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use gpui::{App, Window};
use serde_json::{Map, Value, json};
use specular_interact::{Event, state, update};

use crate::host::{Hosting, ModHost, RunError};
use crate::runner::Mod;

/// What a check saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// The element tree the mod described, one node a line.
    pub tree: String,
    /// The commands it ran while it rendered, `name args` each.
    pub runs: Vec<String>,
    /// The lines it showed with `notify`, `level: text` each.
    pub notices: Vec<String>,
}

/// A host that runs nothing and remembers what it was asked.
#[derive(Debug, Default)]
struct Recorder {
    runs: RefCell<Vec<String>>,
    notices: RefCell<Vec<String>>,
}

impl ModHost for Recorder {
    fn run(&self, name: &str, args: &Value) -> Result<Value, RunError> {
        if specular_interact::command(name).is_none() {
            return Err(RunError(format!("no command `{name}`")));
        }
        self.runs.borrow_mut().push(format!("{name} {args}"));
        Ok(json!({ "ok": true, "name": name }))
    }

    fn commands(&self) -> Value {
        (specular_interact::commands().iter())
            .map(|spec| json!({ "name": spec.name, "args": spec.args, "about": spec.about, "enabled": true }))
            .collect()
    }

    fn notify(&self, level: &str, text: &str) {
        self.notices.borrow_mut().push(format!("{level}: {text}"));
    }
}

/// The keyed state of an app just started in a window that draws its own
/// chrome, as `GET /state` would answer it: the fixture a check uses when it
/// is given none.
pub fn fresh_state() -> Map<String, Value> {
    let mut app = specular_interact::App::new(0);
    update(&mut app, Event::ShellChrome);
    update(&mut app, Event::ChatPanel(true));
    state(&app).into_iter().collect()
}

/// The props the fixture's layout hands the mod `name`: those of the
/// region whose `mod` names it, or null.
fn layout_props(fixture: &Map<String, Value>, name: &str) -> Value {
    let regions = fixture.get("layout").and_then(|layout| layout.get("file"));
    (regions.and_then(Value::as_object).into_iter().flatten())
        .find(|(_, node)| node.get("mod").and_then(Value::as_str) == Some(name))
        .and_then(|(_, node)| node.get("props").cloned())
        .unwrap_or(Value::Null)
}

/// Loads the mod in `dir` over `fixture` (state key to value, as `GET
/// /state` answers; a `mods/<name>/props` key there is its props, else
/// the props its layout gives it) and
/// renders it once in `window`, which it takes over. `data_dir` holds the
/// mod's own state while it runs.
pub fn check(
    dir: &Path,
    data_dir: &Path,
    fixture: &Map<String, Value>,
    window: &mut Window,
    cx: &mut App,
) -> anyhow::Result<CheckReport> {
    let name = (dir.file_name()).map_or_else(
        || "mod".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let recorder = Rc::new(Recorder::default());
    let hosting = Hosting::new(recorder.clone());
    {
        let mut revisions = hosting.revisions.borrow_mut();
        for (key, value) in fixture {
            revisions.publish(key, value.clone());
        }
        let props_key = format!("mods/{name}/props");
        if !fixture.contains_key(&props_key) {
            revisions.publish(&props_key, layout_props(fixture, &name));
        }
    }
    let tree = Mod::check(&name, dir, data_dir, &hosting, window, cx)?;
    Ok(CheckReport {
        tree,
        runs: recorder.runs.take(),
        notices: recorder.notices.take(),
    })
}
