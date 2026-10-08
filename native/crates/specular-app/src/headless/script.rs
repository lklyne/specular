//! The `--script` file: one step of scripted input per line.
//!
//! ```text
//! # select the sticky, then start a marquee and look at it
//! click 240 180
//! press 40 40
//! drag-to 600 420
//! snapshot marquee.png
//! release
//! ```
//!
//! Positions are logical pixels in the snapshot's viewport. A blank line or
//! one starting with `#` is skipped.

use std::path::PathBuf;

use anyhow::{Context as _, bail};
use glam::Vec2;
use specular_core::{Camera, Modifiers};
use specular_interact::{Action, ArrangeMode, Key, Theme, Tool};

/// Where the camera is put.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraArg {
    /// Zoom to fit the whole document, never above 100%.
    Fit,
    /// This pan and zoom.
    At(Camera),
}

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
    /// `control shape.color`: clicks the toolbar or popup control with that
    /// name, wherever it is. A dropdown's options have names once it is
    /// open.
    Control(String),
    /// `hover-control tool.draw`: the pointer moves onto that control.
    HoverControl(String),
    /// `press-control tool.draw`: the button goes down on that control and
    /// stays down until a `release`.
    PressControl(String),
    /// `panels off`, or `panels on`: whether the toolbar and the popup are
    /// drawn and take clicks. They start on.
    Panels(bool),
    /// `sidebar on`, or `sidebar off`: whether the left sidebar is shown. It
    /// starts hidden, and covers the left of the canvas while it is shown.
    Sidebar(bool),
    /// `snapshot out.png`.
    Snapshot(PathBuf),
    /// `save out.canvas`: the text an autosave would write now.
    Save(PathBuf),
}

const NO_MODIFIERS: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    meta: false,
};

/// The steps in `text`, or the first line that is not one.
pub fn parse(text: &str) -> anyhow::Result<Vec<Step>> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|(index, line)| step(line).with_context(|| format!("line {}: `{line}`", index + 1)))
        .collect()
}

fn step(line: &str) -> anyhow::Result<Step> {
    let line = line.trim_start();
    let (verb, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let words: Vec<&str> = rest.split_whitespace().collect();
    Ok(match (verb, words.as_slice()) {
        ("move", [x, y]) => Step::Move(point(x, y)?),
        ("press", [x, y]) => Step::Press(point(x, y)?),
        ("drag-to", [x, y]) => Step::DragTo(point(x, y)?),
        ("release", []) => Step::Release,
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
        ("type", _) if !rest.is_empty() => Step::Type(rest.to_owned()),
        ("compose", _) if !rest.is_empty() => Step::Compose(rest.to_owned()),
        ("commit", _) if !rest.is_empty() => Step::Commit(rest.to_owned()),
        ("clipboard", _) if !rest.is_empty() => Step::Clipboard(unescaped(rest)),
        ("wheel", [x, y]) => Step::Wheel(point(x, y)?),
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
        ("snapshot", [path]) => Step::Snapshot(PathBuf::from(path)),
        ("save", [path]) => Step::Save(PathBuf::from(path)),
        _ => bail!("not a step, or the wrong number of arguments"),
    })
}

/// `text` with `\n`, `\t` and `\\` turned into what they name.
fn unescaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

fn point(x: &str, y: &str) -> anyhow::Result<Vec2> {
    let parse = |value: &str| {
        value
            .parse::<f32>()
            .with_context(|| format!("`{value}` is not a number"))
    };
    Ok(Vec2::new(parse(x)?, parse(y)?))
}

/// `fit`, or `x,y,zoom`.
pub(crate) fn camera(value: &str) -> anyhow::Result<CameraArg> {
    if value == "fit" {
        return Ok(CameraArg::Fit);
    }
    let parts: Vec<f32> = value
        .split(',')
        .map(|part| part.trim().parse())
        .collect::<Result<_, _>>()
        .with_context(|| format!("a camera is `fit` or `x,y,zoom`, got `{value}`"))?;
    match parts.as_slice() {
        [x, y, zoom] => Ok(CameraArg::At(Camera::new(Vec2::new(*x, *y), *zoom))),
        _ => bail!("a camera is `fit` or `x,y,zoom`, got `{value}`"),
    }
}

/// Modifier names joined by `+`. Empty and `none` hold nothing.
fn modifiers(chord: &str) -> anyhow::Result<Modifiers> {
    let mut held = NO_MODIFIERS;
    for name in chord.split('+').filter(|name| !name.is_empty()) {
        match name {
            "none" => {}
            "shift" => held.shift = true,
            "ctrl" | "control" => held.control = true,
            "alt" | "option" => held.alt = true,
            "cmd" | "meta" => held.meta = true,
            other => bail!("unknown modifier `{other}`"),
        }
    }
    Ok(held)
}

fn key_named(name: &str) -> anyhow::Result<Key> {
    let mut characters = name.chars();
    Ok(match name {
        "escape" | "esc" => Key::Escape,
        "enter" | "return" => Key::Enter,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "space" => Key::Space,
        "left" => Key::ArrowLeft,
        "right" => Key::ArrowRight,
        "up" => Key::ArrowUp,
        "down" => Key::ArrowDown,
        _ => match (characters.next(), characters.next()) {
            (Some(character), None) => Key::Char(character.to_ascii_lowercase()),
            _ => bail!("unknown key `{name}`"),
        },
    })
}

/// The theme `name` names: `light`, `dark` or `system`.
pub(crate) fn theme_named(name: &str) -> anyhow::Result<Theme> {
    match name {
        "light" | "dark" | "system" => Ok(Theme::from_key(name)),
        _ => bail!("unknown theme `{name}` (expected light, dark or system)"),
    }
}

fn tool_named(name: &str) -> anyhow::Result<Tool> {
    Ok(match name {
        "select" => Tool::Select,
        "page" => Tool::AddPage,
        "text" => Tool::AddText,
        "sticky" => Tool::AddSticky,
        "document" => Tool::AddDocument,
        "shape" => Tool::AddShape,
        "draw" => Tool::Draw,
        "comment" => Tool::Comment,
        "inspect" => Tool::Inspect,
        other => bail!("unknown tool `{other}`"),
    })
}

fn action_named(name: &str) -> anyhow::Result<Action> {
    Ok(match name {
        "annotate-selection" => Action::AnnotateSelection,
        "arrange-row" => Action::Arrange(ArrangeMode::Row),
        "arrange-column" => Action::Arrange(ArrangeMode::Column),
        "arrange-grid" => Action::Arrange(ArrangeMode::Grid),
        "focus-selection" => Action::FocusSelection,
        "resolve-comment" => Action::ResolveComment(None),
        "page-back" => Action::PageBack,
        "page-forward" => Action::PageForward,
        "page-reload" => Action::PageReload,
        "page-stop" => Action::PageStop,
        "zoom-to-fit" => Action::ZoomToFit,
        other => bail!("unknown action `{other}`"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_parse_into_steps() {
        let held = Modifiers {
            shift: true,
            meta: true,
            ..NO_MODIFIERS
        };
        let at = CameraArg::At(Camera::new(Vec2::new(40.0, -20.0), 0.5));
        for (script, expected) in [
            (
                "# a marquee\n\n  press 10 20\nrelease\n",
                vec![Step::Press(Vec2::new(10.0, 20.0)), Step::Release],
            ),
            (
                "key cmd+shift+z\nkey escape",
                vec![
                    Step::Key(held, Key::Char('z')),
                    Step::Key(NO_MODIFIERS, Key::Escape),
                ],
            ),
            (
                "type Hello,  world",
                vec![Step::Type("Hello,  world".to_owned())],
            ),
            (
                r"clipboard one\ntwo\\n",
                vec![Step::Clipboard("one\ntwo\\n".to_owned())],
            ),
            (
                "act annotate-selection\nact resolve-comment",
                vec![
                    Step::Act(Action::AnnotateSelection),
                    Step::Act(Action::ResolveComment(None)),
                ],
            ),
            (
                "act arrange-row\nact arrange-column\nact arrange-grid\nact focus-selection",
                vec![
                    Step::Act(Action::Arrange(ArrangeMode::Row)),
                    Step::Act(Action::Arrange(ArrangeMode::Column)),
                    Step::Act(Action::Arrange(ArrangeMode::Grid)),
                    Step::Act(Action::FocusSelection),
                ],
            ),
            (
                "sidebar on\nright-click 10 20",
                vec![Step::Sidebar(true), Step::RightClick(Vec2::new(10.0, 20.0))],
            ),
            (
                "camera fit\ncamera 40,-20,0.5",
                vec![Step::Camera(CameraArg::Fit), Step::Camera(at)],
            ),
            (
                "control shape.color\nhover-control tool.draw\npanels off",
                vec![
                    Step::Control("shape.color".to_owned()),
                    Step::HoverControl("tool.draw".to_owned()),
                    Step::Panels(false),
                ],
            ),
        ] {
            assert_eq!(parse(script).unwrap(), expected, "{script:?}");
        }
    }

    #[test]
    fn pointer_modifier_and_panel_verbs_and_every_tool_parse() {
        for (script, expected) in [
            (
                "move 1 2\npress 3 4\ndrag-to 5 6\nclick 7 8\ndouble-click 9 10\ntriple-click 11 12\n\
                 drag 1 2 3 4\nhold shift+cmd\nhold none\ncompose にほ\ncommit 日本\nwheel 3 -7\n\
                 pinch 0.5\nselect a b\nwait 250\npress-control tool.draw\npanels on\nsidebar off\n\
                 snapshot out.png\nsave out.canvas",
                vec![
                    Step::Move(Vec2::new(1.0, 2.0)),
                    Step::Press(Vec2::new(3.0, 4.0)),
                    Step::DragTo(Vec2::new(5.0, 6.0)),
                    Step::Click(Vec2::new(7.0, 8.0)),
                    Step::DoubleClick(Vec2::new(9.0, 10.0)),
                    Step::TripleClick(Vec2::new(11.0, 12.0)),
                    Step::Drag(Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0)),
                    Step::Hold(Modifiers {
                        shift: true,
                        meta: true,
                        ..NO_MODIFIERS
                    }),
                    Step::Hold(NO_MODIFIERS),
                    Step::Compose("にほ".to_owned()),
                    Step::Commit("日本".to_owned()),
                    Step::Wheel(Vec2::new(3.0, -7.0)),
                    Step::Pinch(0.5),
                    Step::Select(vec!["a".to_owned(), "b".to_owned()]),
                    Step::Wait(250),
                    Step::PressControl("tool.draw".to_owned()),
                    Step::Panels(true),
                    Step::Sidebar(false),
                    Step::Snapshot(PathBuf::from("out.png")),
                    Step::Save(PathBuf::from("out.canvas")),
                ],
            ),
            (
                "tool select\ntool page\ntool text\ntool sticky\ntool document\ntool shape\n\
                 tool draw\ntool comment\ntool inspect",
                vec![
                    Step::Tool(Tool::Select),
                    Step::Tool(Tool::AddPage),
                    Step::Tool(Tool::AddText),
                    Step::Tool(Tool::AddSticky),
                    Step::Tool(Tool::AddDocument),
                    Step::Tool(Tool::AddShape),
                    Step::Tool(Tool::Draw),
                    Step::Tool(Tool::Comment),
                    Step::Tool(Tool::Inspect),
                ],
            ),
        ] {
            assert_eq!(parse(script).unwrap(), expected, "{script:?}");
        }
    }

    #[test]
    fn bad_lines_are_rejected_with_their_number() {
        for script in [
            "act nonsense",
            "panels maybe",
            "sidebar",
            "control",
            "tool hammer",
            "key f13",
            "hold hyper",
        ] {
            assert!(parse(script).is_err(), "{script:?}");
        }
        let error = parse("click 1 2\nclick 1").unwrap_err();
        assert!(format!("{error:#}").starts_with("line 2"));
    }
}
