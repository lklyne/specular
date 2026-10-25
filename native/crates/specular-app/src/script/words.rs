//! The words a step is made of: points, chords, keys, tools, actions.

use anyhow::{Context as _, bail};
use glam::Vec2;
use specular_core::{Camera, Modifiers};
use specular_interact::{Action, ArrangeMode, Key, Theme, Tool};

use super::CameraArg;

/// No modifier held.
pub(super) const NO_MODIFIERS: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    meta: false,
};

/// `text` with `\n`, `\t`, `\s` (a space) and `\\` turned into what they
/// name.
pub(super) fn unescaped(text: &str) -> String {
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
            Some('s') => out.push(' '),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// `text` with line breaks, tabs, backslashes and end spaces written as
/// `unescaped` reads them.
pub(super) fn escaped(text: &str) -> String {
    let inner = text
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    // A line is trimmed when it is read, so a space at either end is
    // written as `\s`.
    let start = inner.len() - inner.trim_start_matches(' ').len();
    let end = inner.trim_end_matches(' ').len().max(start);
    format!(
        "{}{}{}",
        "\\s".repeat(start),
        &inner[start..end],
        "\\s".repeat(inner.len() - end)
    )
}

/// A number, or an error that quotes it.
pub(super) fn number(value: &str) -> anyhow::Result<f32> {
    value
        .parse::<f32>()
        .with_context(|| format!("`{value}` is not a number"))
}

pub(super) fn point(x: &str, y: &str) -> anyhow::Result<Vec2> {
    Ok(Vec2::new(number(x)?, number(y)?))
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
pub(super) fn modifiers(chord: &str) -> anyhow::Result<Modifiers> {
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

pub(super) fn key_named(name: &str) -> anyhow::Result<Key> {
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

/// Each tool and the word a script names it by.
pub(super) const TOOLS: [(&str, Tool); 9] = [
    ("select", Tool::Select),
    ("page", Tool::AddPage),
    ("text", Tool::AddText),
    ("sticky", Tool::AddSticky),
    ("document", Tool::AddDocument),
    ("shape", Tool::AddShape),
    ("draw", Tool::Draw),
    ("comment", Tool::Comment),
    ("inspect", Tool::Inspect),
];

pub(super) fn tool_named(name: &str) -> anyhow::Result<Tool> {
    match TOOLS.iter().find(|(word, _)| *word == name) {
        Some((_, tool)) => Ok(*tool),
        None => bail!("unknown tool `{name}`"),
    }
}

/// The word a script names `tool` by.
pub(super) fn tool_word(tool: Tool) -> &'static str {
    TOOLS
        .iter()
        .find(|(_, named)| *named == tool)
        .map_or("?", |(word, _)| word)
}

pub(super) fn action_named(name: &str) -> anyhow::Result<Action> {
    Ok(match name {
        "annotate-selection" => Action::AnnotateSelection,
        "arrange-row" => Action::Arrange(ArrangeMode::Row),
        "arrange-column" => Action::Arrange(ArrangeMode::Column),
        "arrange-grid" => Action::Arrange(ArrangeMode::Grid),
        "resolve-comment" => Action::ResolveComment(None),
        "page-back" => Action::PageBack,
        "page-forward" => Action::PageForward,
        "page-reload" => Action::PageReload,
        "page-stop" => Action::PageStop,
        "zoom-to-fit" => Action::ZoomToFit,
        "undo" => Action::Undo,
        "redo" => Action::Redo,
        other => bail!("unknown action `{other}`"),
    })
}
