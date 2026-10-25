//! The facts an `expect` step states, read from the rest of its line.
//!
//! ```text
//! expect tool sticky
//! expect selection s1 s2            # in any order; `none` for nothing
//! expect focus canvas               # or `focus page p1`
//! expect editing Ship the beta      # the text being edited; `none`
//! expect showing canvas             # or the id of the page shown alone
//! expect canvas Notes               # the active canvas's name
//! expect zoom 0.25
//! expect camera -400,-900,2         # as the `camera` step sets it
//! expect sidebar on
//! expect menu closed               # the right-click menu
//! expect undo none                  # nothing to undo; `undo some`, `redo none`
//! expect count sticky 2             # entities, edges, comments, canvases, selected, or a kind
//! expect entity s1 kind sticky
//! expect entity s1 rect 100 100 200 200
//! expect entity s1 at 100 100
//! expect entity s1 size 200 200
//! expect entity s1 text Ship the beta
//! expect entity d1 note # Plan     # a Document's markdown
//! expect entity s1 parent g1        # or `parent none`
//! expect entity s1 gone
//! expect saved /nodes/s1/color = "4"
//! expect saved /nodes/s1/pageAnchor absent
//! expect saved /annotations/-1 keys id anchor text
//! expect saved /annotations length 6
//! expect saved same-as start        # or `mark NAME`, or a .canvas path
//! expect saved changed-from start at /nodes/s1/x
//! ```
//!
//! An entity is named by its id, `@selected` (the one selected entity),
//! `@top` (the front-most) or `@N` (the Nth from the back, from 1), which
//! is how a script names what it made: new ids differ from run to run in a
//! window. Each works in a `saved` path too.

use std::path::PathBuf;

use anyhow::{Context as _, bail};
use serde_json::Value;
use specular_interact::Tool;

use super::CameraArg;
use super::words::{camera, number, tool_named, unescaped};

/// One `expect` line.
#[derive(Debug, Clone, PartialEq)]
pub enum Expect {
    /// The tool in hand.
    Tool(Tool),
    /// The selection, as a set of ids. Empty is nothing selected.
    Selection(Vec<String>),
    /// Where the keys go: the canvas, or the entered page.
    Focus(Option<String>),
    /// The text being edited, or `None` when nothing is.
    Editing(Option<String>),
    /// What is shown: the canvas (`None`) or one item alone.
    Showing(Option<String>),
    /// The active canvas's name.
    Canvas(String),
    /// The camera's zoom, within a thousandth.
    Zoom(f32),
    /// The camera's pan in screen pixels and its zoom, within half a pixel
    /// and a thousandth.
    Camera([f32; 3]),
    /// Whether the left sidebar is shown.
    Sidebar(bool),
    /// Whether a context menu is open. A dropdown's list is the shell's
    /// own, so it is not a fact of the app's.
    Menu(bool),
    /// Whether there is something to undo.
    Undo(bool),
    /// Whether there is something to redo.
    Redo(bool),
    /// How many of something the active canvas holds.
    Count(Counted, usize),
    /// A fact about one entity.
    Entity(String, EntityFact),
    /// A fact about the canvas as a save would write it now.
    Saved(String, SavedFact),
    /// The canvas as a save would write it is the same as another.
    SameAs(Reference),
    /// It differs from another at exactly these paths.
    ChangedFrom(Reference, Vec<String>),
    /// A fact about the canvas the pointer works on (see [`super::facts_canvas`]).
    OnCanvas(super::facts_canvas::CanvasFact),
}

/// What an `expect count` counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Counted {
    /// Every entity.
    Entities,
    /// The entities of a kind, in `expect entity .. kind` words.
    Kind(String),
    /// Edges.
    Edges,
    /// Comments.
    Comments,
    /// Canvases in the space.
    Canvases,
    /// Selected items.
    Selected,
}

/// A fact about one entity.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityFact {
    /// `page`, `sticky`, `text`, `shape`, `file`, `document`, `drawing` or
    /// `group`.
    Kind(String),
    /// x, y, width and height, within half a unit.
    Rect([f64; 4]),
    /// x and y.
    At([f64; 2]),
    /// Width and height.
    Size([f64; 2]),
    /// Its text: a text's or sticky's words, a shape's label, a file's path,
    /// a page's address.
    Text(String),
    /// The text a Document holds for the markdown file it shows.
    Note(String),
    /// The group it is in, or `None` for none.
    Parent(Option<String>),
    /// It is not in the document.
    Gone,
}

/// A fact about one place in the saved canvas.
#[derive(Debug, Clone, PartialEq)]
pub enum SavedFact {
    /// The value there, compared as JSON (numbers by value).
    Equals(Value),
    /// Nothing is there.
    Absent,
    /// The object there has exactly these keys.
    Keys(Vec<String>),
    /// The array or object there has this many items.
    Length(usize),
}

/// Another canvas to compare with.
#[derive(Debug, Clone, PartialEq)]
pub enum Reference {
    /// The canvas as the run opened it.
    Start,
    /// What `mark NAME` remembered.
    Mark(String),
    /// A `.canvas` file, relative to the script's folder.
    File(PathBuf),
}

/// The fact `rest` (the line after `expect`) states.
pub(super) fn parse(rest: &str) -> anyhow::Result<Expect> {
    let (what, tail) = split(rest);
    let words: Vec<&str> = tail.split_whitespace().collect();
    Ok(match (what, words.as_slice()) {
        ("tool", [name]) => Expect::Tool(tool_named(name)?),
        ("selection", ["none"]) => Expect::Selection(Vec::new()),
        ("selection", ids) if !ids.is_empty() => {
            Expect::Selection(ids.iter().map(|&id| id.to_owned()).collect())
        }
        ("focus", ["canvas"]) => Expect::Focus(None),
        ("focus", ["page", id]) => Expect::Focus(Some((*id).to_owned())),
        ("editing", ["none"]) => Expect::Editing(None),
        ("editing", _) if !tail.is_empty() => Expect::Editing(Some(unescaped(tail))),
        ("showing", ["canvas"]) => Expect::Showing(None),
        ("showing", [id]) => Expect::Showing(Some((*id).to_owned())),
        ("canvas", _) if !tail.is_empty() => Expect::Canvas(tail.to_owned()),
        ("zoom", [zoom]) => Expect::Zoom(number(zoom)?),
        ("camera", [value]) => match camera(value)? {
            CameraArg::At(camera) => Expect::Camera([camera.pan.x, camera.pan.y, camera.zoom]),
            CameraArg::Fit => bail!("a camera to expect is `x,y,zoom`"),
        },
        ("sidebar", [on]) => Expect::Sidebar(on_off(on, "on", "off")?),
        ("menu", [open]) => Expect::Menu(on_off(open, "open", "closed")?),
        ("undo", [some]) => Expect::Undo(on_off(some, "some", "none")?),
        ("redo", [some]) => Expect::Redo(on_off(some, "some", "none")?),
        ("count", [what, count]) => Expect::Count(counted(what), whole(count)?),
        ("entity", [id, ..]) => {
            let (_, fact) = split(tail);
            Expect::Entity((*id).to_owned(), entity_fact(fact)?)
        }
        ("saved", ["same-as", reference @ ..]) => Expect::SameAs(reference_of(reference)?),
        ("saved", ["changed-from", ..]) => {
            let (_, after) = split(tail);
            let Some((reference, paths)) = after.split_once(" at ") else {
                bail!("expected `changed-from REFERENCE at PATH..`");
            };
            let reference: Vec<&str> = reference.split_whitespace().collect();
            let paths = paths.split_whitespace().map(str::to_owned).collect();
            Expect::ChangedFrom(reference_of(&reference)?, paths)
        }
        ("saved", [path, ..]) if path.starts_with('/') => {
            let (_, fact) = split(tail);
            Expect::Saved((*path).to_owned(), saved_fact(fact)?)
        }
        _ => match super::facts_canvas::fact(what, tail)? {
            Some(fact) => Expect::OnCanvas(fact),
            None => bail!("not a fact `expect` knows, or the wrong arguments"),
        },
    })
}

/// The first word of `text` and the rest, trimmed.
fn split(text: &str) -> (&str, &str) {
    let text = text.trim();
    let (first, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    (first, rest.trim())
}

fn on_off(word: &str, yes: &str, no: &str) -> anyhow::Result<bool> {
    match word {
        _ if word == yes => Ok(true),
        _ if word == no => Ok(false),
        _ => bail!("expected `{yes}` or `{no}`, got `{word}`"),
    }
}

fn whole(word: &str) -> anyhow::Result<usize> {
    word.parse()
        .with_context(|| format!("`{word}` is not a count"))
}

fn counted(word: &str) -> Counted {
    match word {
        "entities" => Counted::Entities,
        "edges" => Counted::Edges,
        "comments" => Counted::Comments,
        "canvases" => Counted::Canvases,
        "selected" => Counted::Selected,
        kind => Counted::Kind(kind.to_owned()),
    }
}

fn numbers<const N: usize>(words: &[&str]) -> anyhow::Result<[f64; N]> {
    if words.len() != N {
        bail!("expected {N} numbers");
    }
    let mut out = [0.0; N];
    for (slot, word) in out.iter_mut().zip(words) {
        *slot = f64::from(number(word)?);
    }
    Ok(out)
}

fn entity_fact(text: &str) -> anyhow::Result<EntityFact> {
    let (what, rest) = split(text);
    let words: Vec<&str> = rest.split_whitespace().collect();
    Ok(match (what, words.as_slice()) {
        ("kind", [kind]) => EntityFact::Kind((*kind).to_owned()),
        ("rect", _) => EntityFact::Rect(numbers(&words)?),
        ("at", _) => EntityFact::At(numbers(&words)?),
        ("size", _) => EntityFact::Size(numbers(&words)?),
        ("text", _) => EntityFact::Text(unescaped(rest)),
        ("note", _) => EntityFact::Note(unescaped(rest)),
        ("parent", ["none"]) => EntityFact::Parent(None),
        ("parent", [id]) => EntityFact::Parent(Some((*id).to_owned())),
        ("gone", []) => EntityFact::Gone,
        _ => bail!("an entity fact is kind, rect, at, size, text, note, parent or gone"),
    })
}

fn saved_fact(text: &str) -> anyhow::Result<SavedFact> {
    let (what, rest) = split(text);
    Ok(match what {
        "=" => SavedFact::Equals(
            serde_json::from_str(rest).unwrap_or_else(|_| Value::String(rest.to_owned())),
        ),
        "absent" if rest.is_empty() => SavedFact::Absent,
        "keys" => SavedFact::Keys(rest.split_whitespace().map(str::to_owned).collect()),
        "length" => SavedFact::Length(whole(rest)?),
        _ => bail!("a saved fact is `= VALUE`, `absent`, `keys K..` or `length N`"),
    })
}

fn reference_of(words: &[&str]) -> anyhow::Result<Reference> {
    Ok(match words {
        ["start"] => Reference::Start,
        ["mark", name] => Reference::Mark((*name).to_owned()),
        [path] if path.ends_with(".canvas") => Reference::File(PathBuf::from(path)),
        _ => bail!("compare with `start`, `mark NAME` or a .canvas path"),
    })
}
