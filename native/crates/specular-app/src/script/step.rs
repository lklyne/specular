//! One line of a script as a [`Step`].

use std::path::PathBuf;

use anyhow::{Context as _, bail};
use glam::Vec2;
use specular_core::Modifiers;
use specular_interact::{Action, Key, Tool};

use super::CameraArg;
use super::expect::{self, Expect};
use super::facts_panels::{self, PanelStep};
use super::window::{self, WindowStep};
use super::words::{
    action_named, camera, key_named, modifiers, point, theme_named, tool_named, unescaped,
};

/// One line of a script.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// `move x y`: the pointer moves with no button down.
    Move(Vec2),
    /// `press x y`: the left button goes down.
    Press(Vec2),
    /// `drag-to x y`: the pointer moves with the button still down.
    DragTo(Vec2),
    /// `release`: the left button comes up where the pointer is.
    Release,
    /// `release x y`: the pointer moves there and the button comes up.
    ReleaseAt(Vec2),
    /// `click x y`.
    Click(Vec2),
    /// `double-click x y`.
    DoubleClick(Vec2),
    /// `triple-click x y`.
    TripleClick(Vec2),
    /// `right-click x y`: the secondary button, pressed and released.
    RightClick(Vec2),
    /// `drag x1 y1 x2 y2`: press, move through the middle, release.
    Drag(Vec2, Vec2),
    /// `hold shift+cmd`, or `hold none`: modifiers kept down for the steps
    /// that follow.
    Hold(Modifiers),
    /// `key cmd+shift+z`: one key, pressed and released.
    Key(Modifiers, Key),
    /// `type some text`: the rest of the line, one key a character.
    Type(String),
    /// `compose にほ`: the input method shows this as its composition.
    Compose(String),
    /// `commit 日本`: the input method commits this, ending any composition.
    /// Also how an emoji picker's choice arrives.
    Commit(String),
    /// `clipboard some text`: another app copied this. `\n`, `\t` and `\\`
    /// are a line break, a tab and a backslash. `key cmd+v` pastes it.
    Clipboard(String),
    /// `wheel dx dy`: scrolls where the pointer is, with the held modifiers.
    Wheel(Vec2),
    /// `scroll x y dx dy`: the pointer moves there and scrolls.
    ScrollAt(Vec2, Vec2),
    /// `pinch delta`: a trackpad pinch where the pointer is.
    Pinch(f32),
    /// `tool shape`.
    Tool(Tool),
    /// `select id id`: replaces the selection.
    Select(Vec<String>),
    /// `act annotate-selection` or `act resolve-comment`: the commands
    /// that have a menu item and no key.
    Act(Action),
    /// `camera fit` or `camera x,y,zoom`.
    Camera(CameraArg),
    /// `wait ms`: the clock moves on and pending loads are waited for.
    Wait(u64),
    /// `control shape.color`: clicks the toolbar or dock control with that
    /// name, wherever it is. A dropdown's options have names once it is
    /// open.
    Control(String),
    /// `hover-control tool.draw`: the pointer moves onto that control.
    HoverControl(String),
    /// `press-control tool.draw`: the button goes down on that control and
    /// stays down until a `release`.
    PressControl(String),
    /// `panels off`, or `panels on`: whether the built-in toolbar and dock
    /// are drawn and take clicks. The Kit always draws its own, so a window
    /// skips this step.
    Panels(bool),
    /// `sidebar on`, or `sidebar off`: whether the left sidebar is shown. It
    /// starts hidden, and covers the left of the canvas while it is shown.
    Sidebar(bool),
    /// `motion on`, or `motion off`: whether camera transitions run, as in
    /// a window. They start off headless, so a script's camera lands at once.
    Motion(bool),
    /// `reduce-motion on`, or `reduce-motion off`: the system's Reduce
    /// motion setting, as the shell reads it.
    ReduceMotion(bool),
    /// `page-opens-tab p1 https://example.com/b`, or with `background`
    /// after the address: the page `p1` asks for a link to open in a new
    /// tab, as a `target=_blank` click (or a Command-click) does.
    PageOpensTab {
        /// The page entity.
        page: String,
        /// The address.
        url: String,
        /// Whether the tab would come to the front.
        foreground: bool,
    },
    /// `snapshot out.png` (or `shot out.png`): a picture of what is drawn.
    Snapshot(PathBuf),
    /// `save out.canvas`: the text an autosave would write now.
    Save(PathBuf),
    /// `state out.txt`: the state the `expect` steps read, one fact a line
    /// in their words.
    State(PathBuf),
    /// `expect ...`: a fact about the state or the saved canvas. The run
    /// fails at the first one that does not hold.
    Expect(Expect),
    /// `mark NAME`: remembers the canvas as a save would write it now, for
    /// `expect saved same-as mark NAME`.
    Mark(String),
    /// `reopen`: the canvas is written as a save would write it and read
    /// back, as a relaunch would open it.
    Reopen,
    /// `undo-to-start`: undoes until there is nothing left to undo, then
    /// checks the canvas is the one the run opened. `undo-to-start except
    /// PATH..` names what undo does not take back, such as a page's
    /// address after it navigated: the canvas differs from the start at
    /// exactly those paths.
    UndoToStart(Vec<String>),
    /// The shell's or the agent's side of a panel: see [`PanelStep`].
    Panel(PanelStep),
    /// A step only a window can do. A headless run skips it and says so.
    Window(WindowStep),
}

impl Step {
    /// Whether only a window can do this step.
    pub fn window_only(&self) -> bool {
        matches!(self, Self::Window(_))
    }
}

/// The step `line` names. `line` is not blank and not a comment.
pub(super) fn step(line: &str) -> anyhow::Result<Step> {
    let line = line.trim_start();
    // What follows `type` and the other text verbs is kept as written,
    // spaces at either end included.
    let (verb, text) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = text.trim();
    let words: Vec<&str> = rest.split_whitespace().collect();
    if let Some(step) = window::step(verb, rest)? {
        return Ok(Step::Window(step));
    }
    if let Some(step) = facts_panels::parse_step(verb, text) {
        return Ok(Step::Panel(step?));
    }
    Ok(match (verb, words.as_slice()) {
        ("move", [x, y]) => Step::Move(point(x, y)?),
        ("press", [x, y]) => Step::Press(point(x, y)?),
        ("drag-to", [x, y]) => Step::DragTo(point(x, y)?),
        ("release", []) => Step::Release,
        ("release", [x, y]) => Step::ReleaseAt(point(x, y)?),
        ("click", [x, y]) => Step::Click(point(x, y)?),
        ("double-click", [x, y]) => Step::DoubleClick(point(x, y)?),
        ("triple-click", [x, y]) => Step::TripleClick(point(x, y)?),
        ("right-click", [x, y]) => Step::RightClick(point(x, y)?),
        ("drag", [x1, y1, x2, y2]) => Step::Drag(point(x1, y1)?, point(x2, y2)?),
        ("hold", [chord]) => Step::Hold(modifiers(chord)?),
        ("key", [chord]) => {
            let (held, key) = chord.rsplit_once('+').unwrap_or(("", chord));
            Step::Key(modifiers(held)?, key_named(key)?)
        }
        ("type", _) if !text.is_empty() => Step::Type(text.to_owned()),
        ("compose", _) if !text.is_empty() => Step::Compose(text.to_owned()),
        ("commit", _) if !text.is_empty() => Step::Commit(text.to_owned()),
        ("clipboard", _) if !rest.is_empty() => Step::Clipboard(unescaped(rest)),
        ("wheel", [x, y]) => Step::Wheel(point(x, y)?),
        ("scroll", [x, y, dx, dy]) => Step::ScrollAt(point(x, y)?, point(dx, dy)?),
        ("pinch", [delta]) => Step::Pinch(delta.parse().context("pinch expects a number")?),
        ("tool", [name]) => Step::Tool(tool_named(name)?),
        ("act", [name]) => Step::Act(action_named(name)?),
        ("theme", [name]) => Step::Act(Action::SetTheme(theme_named(name)?)),
        ("select", ids) => Step::Select(ids.iter().map(|&id| id.to_owned()).collect()),
        ("camera", [value]) => Step::Camera(camera(value)?),
        ("wait", [ms]) => Step::Wait(ms.parse().context("wait expects milliseconds")?),
        ("control", [id]) => Step::Control((*id).to_owned()),
        ("hover-control", [id]) => Step::HoverControl((*id).to_owned()),
        ("press-control", [id]) => Step::PressControl((*id).to_owned()),
        ("panels", ["on"]) => Step::Panels(true),
        ("panels", ["off"]) => Step::Panels(false),
        ("sidebar", ["on"]) => Step::Sidebar(true),
        ("sidebar", ["off"]) => Step::Sidebar(false),
        ("motion", [on]) => Step::Motion(switch(on)?),
        ("reduce-motion", [on]) => Step::ReduceMotion(switch(on)?),
        ("page-opens-tab", [page, url]) => Step::PageOpensTab {
            page: (*page).to_owned(),
            url: (*url).to_owned(),
            foreground: true,
        },
        ("page-opens-tab", [page, url, "background"]) => Step::PageOpensTab {
            page: (*page).to_owned(),
            url: (*url).to_owned(),
            foreground: false,
        },
        ("snapshot" | "shot", [path]) => Step::Snapshot(PathBuf::from(path)),
        ("save", [path]) => Step::Save(PathBuf::from(path)),
        ("state", [path]) => Step::State(PathBuf::from(path)),
        ("expect", _) if !rest.is_empty() => Step::Expect(expect::parse(rest)?),
        ("mark", [name]) => Step::Mark((*name).to_owned()),
        ("reopen", []) => Step::Reopen,
        ("undo-to-start", []) => Step::UndoToStart(Vec::new()),
        ("undo-to-start", ["except", paths @ ..]) if !paths.is_empty() => {
            Step::UndoToStart(paths.iter().map(|&path| path.to_owned()).collect())
        }
        _ => bail!("not a step, or the wrong number of arguments"),
    })
}

/// `on` or `off`.
fn switch(word: &str) -> anyhow::Result<bool> {
    match word {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => bail!("expected `on` or `off`, got `{word}`"),
    }
}
