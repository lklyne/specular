//! Whether an `expect` holds, read from the app alone, so a headless run and
//! a window check the same thing the same way.

mod entity;
mod saved;

use self::entity::{counted, counted_name, entity_fact};
pub(crate) use self::saved::saved_text;
use self::saved::{saved_fact, saved_json, saved_json_of};

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, anyhow, bail, ensure};
use serde_json::Value;
use specular_doc::{Entity, Kind, TextStyle};
use specular_interact::{App, CanvasId, Focus, Showing};

use super::expect::{Expect, Reference};
use super::saved::{canonical, differences, listed};
use super::words::{escaped, tool_word};

/// How far apart two coordinates may be and still be the same.
const NEAR: f64 = 0.5;

/// What the checks of one run remember: the canvases it opened and the
/// ones it marked.
#[derive(Debug, Default)]
pub struct Checks {
    /// The folder a `same-as` path starts from: the script's.
    base: PathBuf,
    start: HashMap<CanvasId, Value>,
    marks: HashMap<String, Value>,
}

impl Checks {
    /// Remembers every canvas of `app` as it is now, as the start that
    /// `undo-to-start` and `same-as start` compare with. `base` is the
    /// script's folder.
    pub fn begin(app: &App, base: &Path) -> anyhow::Result<Self> {
        let mut start = HashMap::new();
        for canvas in app.space().canvases() {
            start.insert(canvas.id.clone(), saved_json_of(app, &canvas.id)?);
        }
        Ok(Self {
            base: base.to_owned(),
            start,
            marks: HashMap::new(),
        })
    }

    /// Remembers the active canvas as `name`.
    pub fn mark(&mut self, app: &App, name: &str) -> anyhow::Result<()> {
        self.marks.insert(name.to_owned(), saved_json(app)?);
        Ok(())
    }

    /// Whether the active canvas is the one the run opened, but for
    /// `except`, the paths undo does not take back. Run once nothing is
    /// left to undo.
    pub fn back_at_start(&self, app: &App, except: &[String]) -> anyhow::Result<()> {
        ensure!(
            !app.can_undo(),
            "undo did not run out of steps; a gesture or an edit may still be open"
        );
        let checked = if except.is_empty() {
            self.same_as(app, &Reference::Start)
        } else {
            self.changed_from(app, &Reference::Start, except)
        };
        checked.context("undoing every step did not return to the start")
    }

    /// Whether `expect` holds for `app`. The error says what was found.
    pub fn check(&self, app: &App, expect: &Expect) -> anyhow::Result<()> {
        let session = app.session();
        match expect {
            Expect::Tool(tool) => equal("tool", tool_word(session.tool), tool_word(*tool)),
            Expect::Selection(ids) => {
                let mut found: Vec<String> = (session.selection.items().iter())
                    .map(|item| item.as_str().to_owned())
                    .collect();
                let mut wanted: Vec<String> = (ids.iter())
                    .map(|id| resolve(app, id))
                    .collect::<anyhow::Result<_>>()?;
                found.sort();
                wanted.sort();
                equal("selection", words(&found), words(&wanted))
            }
            Expect::Focus(page) => {
                let found = match &session.focus {
                    Focus::Canvas => "canvas".to_owned(),
                    Focus::Page(id) => format!("page {id}"),
                };
                let wanted = page
                    .as_ref()
                    .map_or("canvas".to_owned(), |id| format!("page {id}"));
                equal("focus", found, wanted)
            }
            Expect::Editing(text) => {
                let found = app.text_edit().map(|edit| escaped(edit.text()));
                let wanted = text.as_deref().map(escaped);
                equal("editing", or_none(found), or_none(wanted))
            }
            Expect::Showing(item) => {
                let found = match app.showing() {
                    Showing::Canvas => "canvas".to_owned(),
                    Showing::Item(id) => id.to_string(),
                };
                equal(
                    "showing",
                    found,
                    item.clone().unwrap_or("canvas".to_owned()),
                )
            }
            Expect::Canvas(name) => equal("canvas", &app.space().active().name, name),
            Expect::Zoom(zoom) => {
                let found = session.camera.zoom;
                ensure!(
                    (found - zoom).abs() <= 0.001,
                    "zoom is {found}, expected {zoom}"
                );
                Ok(())
            }
            Expect::Camera([x, y, zoom]) => {
                let camera = session.camera;
                let near_pan = (camera.pan.x - x).abs() <= 0.5 && (camera.pan.y - y).abs() <= 0.5;
                ensure!(
                    near_pan && (camera.zoom - zoom).abs() <= 0.001,
                    "camera is {},{},{}, expected {x},{y},{zoom}",
                    camera.pan.x,
                    camera.pan.y,
                    camera.zoom
                );
                Ok(())
            }
            Expect::Sidebar(shown) => equal("sidebar", session.sidebar.shown(), *shown),
            Expect::Menu(open) => equal("menu open", session.panel.menu.is_some(), *open),
            Expect::Undo(some) => equal("something to undo", app.can_undo(), *some),
            Expect::Redo(some) => equal("something to redo", app.can_redo(), *some),
            Expect::Count(what, count) => equal(&counted_name(what), counted(app, what), *count),
            Expect::Entity(id, fact) => entity_fact(app, id, fact),
            Expect::Saved(path, fact) => saved_fact(app, path, fact),
            Expect::SameAs(reference) => self.same_as(app, reference),
            Expect::ChangedFrom(reference, paths) => self.changed_from(app, reference, paths),
            Expect::OnCanvas(fact) => super::facts_canvas::check(app, fact),
            Expect::Panel(fact) => fact.check(app, None),
        }
    }

    fn reference(&self, app: &App, reference: &Reference) -> anyhow::Result<Value> {
        Ok(match reference {
            Reference::Start => {
                let id = &app.space().active().id;
                (self.start.get(id).cloned())
                    .with_context(|| format!("the canvas `{id}` was not open when the run began"))?
            }
            Reference::Mark(name) => (self.marks.get(name).cloned())
                .with_context(|| format!("nothing was marked `{name}`"))?,
            Reference::File(path) => {
                let path = self.base.join(path);
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                serde_json::from_str(&text)
                    .with_context(|| format!("parsing {}", path.display()))?
            }
        })
    }

    fn same_as(&self, app: &App, reference: &Reference) -> anyhow::Result<()> {
        let found = differences(&self.reference(app, reference)?, &saved_json(app)?);
        ensure!(
            found.is_empty(),
            "the canvas differs from {}:{}",
            name_of(reference),
            listed(&found)
        );
        Ok(())
    }

    fn changed_from(
        &self,
        app: &App,
        reference: &Reference,
        paths: &[String],
    ) -> anyhow::Result<()> {
        let before = self.reference(app, reference)?;
        let found = differences(&before, &saved_json(app)?);
        let mut changed: Vec<&str> = found.iter().map(|(path, _, _)| path.as_str()).collect();
        let mut wanted: Vec<String> = (paths.iter())
            .map(|path| Ok(canonical(&before, &resolve_path(app, path)?)))
            .collect::<anyhow::Result<_>>()?;
        changed.sort_unstable();
        wanted.sort();
        ensure!(
            changed == wanted,
            "expected changes from {} at {} only, found:{}",
            name_of(reference),
            wanted.join(" "),
            listed(&found)
        );
        Ok(())
    }
}

fn name_of(reference: &Reference) -> String {
    match reference {
        Reference::Start => "the start".to_owned(),
        Reference::Mark(name) => format!("the mark `{name}`"),
        Reference::File(path) => path.display().to_string(),
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "callers hand over the values they just worked out"
)]
fn equal<T: PartialEq + std::fmt::Display>(what: &str, found: T, wanted: T) -> anyhow::Result<()> {
    ensure!(found == wanted, "{what} is {found}, expected {wanted}");
    Ok(())
}

fn words(ids: &[String]) -> String {
    if ids.is_empty() {
        "none".to_owned()
    } else {
        ids.join(" ")
    }
}

fn or_none(text: Option<String>) -> String {
    text.unwrap_or_else(|| "none".to_owned())
}

/// The id `name` stands for: itself, or the entity `@selected`, `@top` or
/// `@N` (the Nth from the back, from 1) is now.
pub(super) fn resolve(app: &App, name: &str) -> anyhow::Result<String> {
    match name {
        "@selected" => {
            let mut selected = app.session().selection.entities();
            match (selected.next(), selected.next()) {
                (Some(id), None) => Ok(id.to_string()),
                _ => bail!("`@selected` needs exactly one selected entity"),
            }
        }
        "@top" => (app.document().entities().last())
            .map(|entity| entity.id.to_string())
            .ok_or_else(|| anyhow!("`@top` needs an entity")),
        _ => match name.strip_prefix('@').map(str::parse::<usize>) {
            Some(Ok(place)) => (app.document().entities().nth(place.wrapping_sub(1)))
                .map(|entity| entity.id.to_string())
                .ok_or_else(|| anyhow!("there is no entity `{name}`")),
            _ => Ok(name.to_owned()),
        },
    }
}

fn resolve_path(app: &App, path: &str) -> anyhow::Result<String> {
    let steps: Vec<String> = (path.split('/'))
        .map(|step| {
            if step.starts_with('@') {
                resolve(app, step)
            } else {
                Ok(step.to_owned())
            }
        })
        .collect::<anyhow::Result<_>>()?;
    Ok(steps.join("/"))
}

/// The word a script names `entity`'s kind by.
pub(super) fn kind_word(entity: &Entity) -> &'static str {
    match &entity.kind {
        Kind::Text(text) if text.resolved_style() == TextStyle::Sticky => "sticky",
        Kind::File(file) if is_markdown(&file.file) => "document",
        kind => kind.name(),
    }
}

/// Whether the file at `path` is markdown, which a Document shows.
pub(crate) fn is_markdown(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

/// What `expect entity .. text` reads of `entity`.
pub(super) fn text_of(entity: &Entity) -> &str {
    match &entity.kind {
        Kind::Text(text) => &text.text,
        Kind::Shape(shape) => &shape.text,
        Kind::File(file) => &file.file,
        Kind::Page(page) => &page.url,
        Kind::Group(_) | Kind::Drawing(_) => "",
    }
}

/// The text the document holds for the markdown file `entity` shows.
pub(super) fn note_of<'a>(app: &'a App, entity: &Entity) -> Option<&'a str> {
    match &entity.kind {
        Kind::File(file) => app.document().note(&file.file),
        _ => None,
    }
}
