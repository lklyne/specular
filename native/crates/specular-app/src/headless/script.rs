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
use specular_interact::{Key, Tool};

/// Where the camera is put.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CameraArg {
    /// Zoom to fit the whole document, never above 100%.
    Fit,
    /// This pan and zoom.
    At(Camera),
}

/// One line of a script.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
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
    /// `camera fit` or `camera x,y,zoom`.
    Camera(CameraArg),
    /// `wait ms`: the clock moves on and pending loads are waited for.
    Wait(u64),
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
pub(crate) fn parse(text: &str) -> anyhow::Result<Vec<Step>> {
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
        ("select", ids) => Step::Select(ids.iter().map(|&id| id.to_owned()).collect()),
        ("camera", [value]) => Step::Camera(camera(value)?),
        ("wait", [ms]) => Step::Wait(ms.parse().context("wait expects milliseconds")?),
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
        other => bail!("unknown tool `{other}`"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let steps = parse("# a marquee\n\n  press 10 20\nrelease\n").unwrap();
        assert_eq!(steps, [Step::Press(Vec2::new(10.0, 20.0)), Step::Release]);
    }

    #[test]
    fn a_key_chord_splits_into_modifiers_and_a_key() {
        let held = Modifiers {
            shift: true,
            meta: true,
            ..NO_MODIFIERS
        };
        assert_eq!(
            parse("key cmd+shift+z\nkey escape").unwrap(),
            [
                Step::Key(held, Key::Char('z')),
                Step::Key(NO_MODIFIERS, Key::Escape)
            ]
        );
    }

    #[test]
    fn type_keeps_the_rest_of_the_line() {
        assert_eq!(
            parse("type Hello,  world").unwrap(),
            [Step::Type("Hello,  world".to_owned())]
        );
    }

    #[test]
    fn clipboard_text_takes_escapes() {
        assert_eq!(
            parse(r"clipboard one\ntwo\\n").unwrap(),
            [Step::Clipboard("one\ntwo\\n".to_owned())]
        );
    }

    #[test]
    fn a_camera_is_fit_or_three_numbers() {
        let at = CameraArg::At(Camera::new(Vec2::new(40.0, -20.0), 0.5));
        assert_eq!(
            parse("camera fit\ncamera 40,-20,0.5").unwrap(),
            [Step::Camera(CameraArg::Fit), Step::Camera(at)]
        );
    }

    #[test]
    fn a_bad_line_is_reported_with_its_number() {
        let error = parse("click 1 2\nclick 1").unwrap_err();
        assert!(format!("{error:#}").starts_with("line 2"));
    }

    #[test]
    fn unknown_tools_keys_and_modifiers_are_rejected() {
        assert!(parse("tool hammer").is_err());
        assert!(parse("key f13").is_err());
        assert!(parse("hold hyper").is_err());
    }
}
