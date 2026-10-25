//! The steps only a window can do: the window server's own events, the
//! menu bar, a key by its hardware code. A headless run skips them and
//! lists them when it ends.

use std::path::PathBuf;

use anyhow::bail;
use glam::Vec2;
use specular_core::Modifiers;

use super::words::{NO_MODIFIERS, modifiers, number, point};

/// A step only a window can do.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowStep {
    /// `keycode CODE [CHARS]`, `cmd-key ..`, `shift-key ..`, `keys shift+cmd
    /// CODE [CHARS]`: a key by its `kVK_*` code. `chars` is what it types,
    /// when the line gives it.
    KeyCode {
        /// The `kVK_*` code.
        code: u16,
        /// The modifiers held with it.
        held: Modifiers,
        /// What the key types, when the line says.
        chars: Option<String>,
    },
    /// `pan dx dy ms`: a scroll by that much every refresh for that long,
    /// where the pointer is.
    Pan(Vec2, u64),
    /// `resize-drag dx dy ms`: the window's bottom-right corner dragged by
    /// that much, a move every refresh: `AppKit`'s own live resize.
    ResizeDrag(Vec2, u64),
    /// `swipe dx dy ms [late]`: pressed where the pointer is, dragged by that
    /// much over that long, released `late` ms after the last move.
    Swipe(Vec2, u64, u64),
    /// `drop PATH.. x y`, or `drop PATH.. nowhere`: files dropped. A
    /// headless run drops them too, a relative path read from the canvas's
    /// folder.
    Drop(Vec<PathBuf>, Option<Vec2>),
    /// `paste-image PATH`: the right panel's composer takes the image file
    /// as a paste.
    PasteImage(PathBuf),
    /// `choose NAME`: the first-run view's choice of that name.
    Choose(String),
    /// `resize w h`: the window's content resized, its top-left kept.
    Resize(Vec2),
    /// `window-to x y`: the window's frame corner moved to that screen point.
    MoveWindow(Vec2),
    /// `full-screen`: into full screen, or back out.
    FullScreen,
    /// `menu-dump PATH`: the menu bar as `AppKit` shows it.
    MenuDump(PathBuf),
    /// `menu-choose Menu > Item`: a menu item chosen as a click does.
    MenuChoose(String),
    /// `accessibility PATH`: what the window exposes to assistive apps.
    Accessibility(PathBuf),
    /// `quit`: ends the run.
    Quit,
}

/// The window step `verb` names, or `None` when it is not one of them.
pub(super) fn step(verb: &str, rest: &str) -> anyhow::Result<Option<WindowStep>> {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let path = || {
        if rest.is_empty() {
            bail!("`{verb}` takes a path");
        }
        Ok(PathBuf::from(rest))
    };
    let text = || {
        if rest.is_empty() {
            bail!("`{verb}` takes a name");
        }
        Ok(rest.to_owned())
    };
    let ms = |word: &str| number(word).map(|ms| ms.max(0.0) as u64);
    let step = match (verb, words.as_slice()) {
        ("keycode", [code, chars @ ..]) => key_code(NO_MODIFIERS, code, chars)?,
        ("cmd-key", [code, chars @ ..]) => key_code(held("cmd")?, code, chars)?,
        ("shift-key", [code, chars @ ..]) => key_code(held("shift")?, code, chars)?,
        ("keys", [chord, code, chars @ ..]) => key_code(held(chord)?, code, chars)?,
        ("pan", [dx, dy, time]) => WindowStep::Pan(point(dx, dy)?, ms(time)?),
        ("resize-drag", [dx, dy, time]) => WindowStep::ResizeDrag(point(dx, dy)?, ms(time)?),
        ("swipe", [dx, dy, time]) => WindowStep::Swipe(point(dx, dy)?, ms(time)?, 0),
        ("swipe", [dx, dy, time, late]) => WindowStep::Swipe(point(dx, dy)?, ms(time)?, ms(late)?),
        ("drop", [paths @ .., "nowhere"]) if !paths.is_empty() => {
            WindowStep::Drop(paths.iter().map(PathBuf::from).collect(), None)
        }
        ("drop", [paths @ .., x, y]) if !paths.is_empty() => WindowStep::Drop(
            paths.iter().map(PathBuf::from).collect(),
            Some(point(x, y)?),
        ),
        ("paste-image", _) => WindowStep::PasteImage(path()?),
        ("choose", _) => WindowStep::Choose(text()?),
        ("resize", [w, h]) => WindowStep::Resize(point(w, h)?),
        ("window-to", [x, y]) => WindowStep::MoveWindow(point(x, y)?),
        ("full-screen", []) => WindowStep::FullScreen,
        ("menu-dump", _) => WindowStep::MenuDump(path()?),
        ("menu-choose", _) => WindowStep::MenuChoose(text()?),
        ("accessibility", _) => WindowStep::Accessibility(path()?),
        ("quit", []) => WindowStep::Quit,
        (
            "keycode" | "cmd-key" | "shift-key" | "keys" | "pan" | "resize-drag" | "swipe" | "drop"
            | "resize" | "window-to" | "full-screen" | "quit",
            _,
        ) => bail!("the wrong arguments"),
        _ => return Ok(None),
    };
    Ok(Some(step))
}

fn held(chord: &str) -> anyhow::Result<Modifiers> {
    modifiers(chord)
}

fn key_code(held: Modifiers, code: &str, chars: &[&str]) -> anyhow::Result<WindowStep> {
    let Ok(code) = code.parse::<u16>() else {
        bail!("`{code}` is not a key code");
    };
    Ok(WindowStep::KeyCode {
        code,
        held,
        chars: chars.first().map(|&chars| chars.to_owned()),
    })
}
