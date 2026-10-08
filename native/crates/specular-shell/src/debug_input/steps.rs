//! A script's text as the things the window runner does.
//!
//! Two vocabularies meet here. The headless runner's (`--script`, one step a
//! line, read by `specular_app::script::parse`) is taken as it is, so a
//! scenario file runs in the window as written. The window's own steps are
//! the ones a headless run has no use for: a capture of the real window,
//! the menu bar, a key by its code. Nothing is skipped: a step with no
//! equivalent in the window is an error that names it.

use std::path::PathBuf;

use glam::Vec2;
use specular_app::script::{self, CameraArg};
use specular_interact::Action;

use super::us_keys::{COMMAND, CONTROL, OPTION, SHIFT, flags_of, key_of, key_step};

/// Where a pointer step happens.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Place {
    /// A point, in points from the content's top-left corner.
    At(Vec2),
    /// The middle of the control with this name, wherever it is drawn.
    Control(String),
    /// Where the pointer was last put.
    Pointer,
}

/// One thing the runner does in the live window.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Do {
    Wait(u64),
    Move(Place),
    /// The left button goes down and stays down.
    Press(Place),
    /// The pointer moves with the button down.
    DragTo(Vec2),
    Release(Place),
    /// This many clicks in a row: one, a double or a triple.
    Click(Place, u8),
    RightClick(Vec2),
    /// Press, move through the middle, release.
    Drag(Vec2, Vec2),
    /// Modifier flags for the pointer and scroll steps after it.
    Hold(usize),
    /// A key by its `kVK_*` code, pressed and released.
    Key {
        code: u16,
        flags: usize,
        /// `NSEvent.characters`.
        characters: String,
        /// `NSEvent.charactersIgnoringModifiers`.
        plain: String,
    },
    Type(String),
    /// The input method shows this as its composition.
    Compose(String),
    /// The input method commits this.
    Commit(String),
    /// Another app copied this, into the run's own clipboard.
    Clipboard(String),
    /// A pixel scroll.
    Scroll(Place, Vec2),
    /// A scroll by this much every refresh for this many milliseconds,
    /// where the pointer is: a pan to read the frame log under.
    Pan(Vec2, u64),
    /// A trackpad pinch where the pointer is.
    Pinch(f32),
    Act(Action),
    Select(Vec<String>),
    Sidebar(bool),
    /// The right panel's composer is handed this image file as a paste.
    PasteImage(PathBuf),
    /// The first-run view's choice with this name, run as its button runs it.
    Choose(String),
    /// Files dropped at a point, or where a paste would put them.
    Drop(Vec<PathBuf>, Option<Vec2>),
    /// The window's content resized to this many points.
    Resize(Vec2),
    /// Into full screen, or back out.
    FullScreen,
    MenuDump(PathBuf),
    MenuChoose(String),
    /// The app's state as text: selection, camera, entities, window.
    State(PathBuf),
    /// What the window exposes to assistive apps, as text.
    Accessibility(PathBuf),
    /// The window as the window server composites it.
    Shot(PathBuf),
    /// The `.canvas` text an autosave would write.
    Save(PathBuf),
    Quit,
}

/// How a script's text is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dialect {
    /// `SPECULAR_SHELL_SCRIPT`: `key 51` is a key code, as it always was.
    Inline,
    /// A script file: every headless step means what it means headless.
    File,
}

/// A headless step as what the window does for it.
fn lower(step: script::Step) -> Result<Do, String> {
    use script::Step;
    Ok(match step {
        Step::Move(at) => Do::Move(Place::At(at)),
        Step::Press(at) => Do::Press(Place::At(at)),
        Step::DragTo(at) => Do::DragTo(at),
        Step::Release => Do::Release(Place::Pointer),
        Step::Click(at) => Do::Click(Place::At(at), 1),
        Step::DoubleClick(at) => Do::Click(Place::At(at), 2),
        Step::TripleClick(at) => Do::Click(Place::At(at), 3),
        Step::RightClick(at) => Do::RightClick(at),
        Step::Drag(from, to) => Do::Drag(from, to),
        Step::Hold(held) => Do::Hold(flags_of(held)),
        Step::Key(held, key) => key_step(held, key)?,
        Step::Type(text) => Do::Type(text),
        Step::Compose(text) => Do::Compose(text),
        Step::Commit(text) => Do::Commit(text),
        Step::Clipboard(text) => Do::Clipboard(text),
        Step::Wheel(delta) => Do::Scroll(Place::Pointer, delta),
        Step::Pinch(delta) => Do::Pinch(delta),
        Step::Tool(tool) => Do::Act(Action::SetTool(tool)),
        Step::Select(ids) => Do::Select(ids),
        Step::Act(action) => Do::Act(action),
        Step::Camera(CameraArg::Fit) => Do::Act(Action::ZoomToFit),
        Step::Camera(CameraArg::At(camera)) => Do::Act(Action::SetCamera(camera)),
        Step::Wait(ms) => Do::Wait(ms),
        Step::Control(name) => Do::Click(Place::Control(name), 1),
        Step::HoverControl(name) => Do::Move(Place::Control(name)),
        Step::PressControl(name) => Do::Press(Place::Control(name)),
        Step::Panels(_) => {
            return Err(
                "`panels` has no equivalent in this window: the Kit's toolbar and sidebar \
                 are always drawn"
                    .to_owned(),
            );
        }
        Step::Sidebar(shown) => Do::Sidebar(shown),
        Step::Snapshot(path) => Do::Shot(path),
        Step::Save(path) => Do::Save(path),
    })
}

/// Modifier flags from `shift+cmd`.
fn flags(names: &str) -> Option<usize> {
    names.split('+').try_fold(0, |flags, name| {
        Some(
            flags
                | match name {
                    "shift" => SHIFT,
                    "ctrl" => CONTROL,
                    "alt" => OPTION,
                    "cmd" => COMMAND,
                    _ => return None,
                },
        )
    })
}

/// The characters `AppKit` gives a key that types none of its own. GPUI
/// names a key by them, so Return without `\r` is no key at all.
const fn named_characters(code: u16) -> Option<&'static str> {
    match code {
        36 | 76 => Some("\r"),
        48 => Some("\t"),
        49 => Some(" "),
        51 => Some("\u{7f}"),
        53 => Some("\u{1b}"),
        123 => Some("\u{f702}"),
        124 => Some("\u{f703}"),
        125 => Some("\u{f701}"),
        126 => Some("\u{f700}"),
        _ => None,
    }
}

/// A step only the window has, or `None` when `verb` is not one of them.
fn window_step(verb: &str, rest: &str, dialect: Dialect) -> Option<Result<Do, String>> {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let number = |at: usize| words.get(at).and_then(|word| word.parse::<f32>().ok());
    let point = |at: usize| Some(Vec2::new(number(at)?, number(at + 1)?));
    let path = || (!rest.is_empty()).then(|| PathBuf::from(rest));
    let coded = |held: usize, at: usize| {
        let code = words.get(at)?.parse::<u16>().ok()?;
        let characters = match (words.get(at + 1), named_characters(code)) {
            (Some(given), _) => given,
            (None, Some(named)) => named,
            (None, None) => "",
        }
        .to_owned();
        Some(Do::Key {
            code,
            flags: held,
            plain: characters.clone(),
            characters,
        })
    };
    let step = match verb {
        "release" if words.len() == 2 => point(0).map(|at| Do::Release(Place::At(at))),
        "scroll" => point(0)
            .zip(point(2))
            .map(|(at, by)| Do::Scroll(Place::At(at), by)),
        "pan" => point(0)
            .zip(number(2))
            .map(|(by, ms)| Do::Pan(by, ms as u64)),
        "key" if dialect == Dialect::Inline && number(0).is_some() => coded(0, 0),
        "keycode" => coded(0, 0),
        "cmd-key" => coded(COMMAND, 0),
        "shift-key" => coded(SHIFT, 0),
        "keys" => words
            .first()
            .and_then(|held| flags(held))
            .and_then(|held| coded(held, 1)),
        "drop" => {
            let (paths, at) = match words.as_slice() {
                [paths @ .., "nowhere"] => (paths, Some(None)),
                [paths @ .., _, _] => (paths, point(paths.len()).map(Some)),
                _ => (&[][..], None),
            };
            at.filter(|_| !paths.is_empty())
                .map(|at| Do::Drop(paths.iter().map(PathBuf::from).collect(), at))
        }
        "paste-image" => path().map(Do::PasteImage),
        "choose" => (!rest.is_empty()).then(|| Do::Choose(rest.to_owned())),
        "resize" => point(0).map(Do::Resize),
        "full-screen" => Some(Do::FullScreen),
        "menu-dump" => path().map(Do::MenuDump),
        "menu-choose" => (!rest.is_empty()).then(|| Do::MenuChoose(rest.to_owned())),
        "state" => path().map(Do::State),
        "accessibility" => path().map(Do::Accessibility),
        "shot" => path().map(Do::Shot),
        "quit" => Some(Do::Quit),
        _ => return None,
    };
    Some(step.ok_or_else(|| "the wrong arguments".to_owned()))
}

/// One step of a script.
pub(super) fn parse(text: &str, dialect: Dialect) -> Result<Do, String> {
    let (verb, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    if let Some(step) = window_step(verb, rest.trim(), dialect) {
        return step;
    }
    let mut steps = script::parse(text).map_err(|error| format!("{:#}", error.root_cause()))?;
    match (steps.pop(), steps.is_empty()) {
        (Some(step), true) => lower(step),
        _ => Err("not one step".to_owned()),
    }
}

/// Whether `step` copies, cuts or pastes by its key.
pub(super) fn is_clipboard_key(step: &Do) -> bool {
    let keys = ['c', 'x', 'v'].map(|key| key_of(key).map(|(code, _)| code));
    matches!(step, Do::Key { code, flags, .. }
        if flags & COMMAND != 0 && keys.contains(&Some(*code)))
}

#[cfg(test)]
mod tests {
    use specular_core::Camera;
    use specular_interact::Tool;

    use super::*;

    fn key(code: u16, flags: usize, characters: &str, plain: &str) -> Do {
        Do::Key {
            code,
            flags,
            characters: characters.to_owned(),
            plain: plain.to_owned(),
        }
    }

    #[test]
    fn every_headless_step_is_something_the_window_does_or_an_error_that_names_it() {
        let at = Vec2::new(10.0, 20.0);
        let there = Place::At(at);
        let named = |name: &str| Place::Control(name.to_owned());
        let cases = [
            ("move 10 20", Do::Move(there.clone())),
            ("press 10 20", Do::Press(there.clone())),
            ("drag-to 10 20", Do::DragTo(at)),
            ("release", Do::Release(Place::Pointer)),
            ("click 10 20", Do::Click(there.clone(), 1)),
            ("double-click 10 20", Do::Click(there.clone(), 2)),
            ("triple-click 10 20", Do::Click(there, 3)),
            ("right-click 10 20", Do::RightClick(at)),
            ("drag 10 20 30 40", Do::Drag(at, Vec2::new(30.0, 40.0))),
            ("hold shift+cmd", Do::Hold(SHIFT | COMMAND)),
            ("hold none", Do::Hold(0)),
            ("key cmd+z", key(6, COMMAND, "z", "z")),
            ("key cmd+shift+z", key(6, COMMAND | SHIFT, "z", "Z")),
            ("key shift+tab", key(48, SHIFT, "\t", "\t")),
            ("key backspace", key(51, 0, "\u{7f}", "\u{7f}")),
            ("key 5", key(23, 0, "5", "5")),
            ("type Hi there", Do::Type("Hi there".to_owned())),
            ("compose にほ", Do::Compose("にほ".to_owned())),
            ("commit 日本", Do::Commit("日本".to_owned())),
            (r"clipboard a\nb", Do::Clipboard("a\nb".to_owned())),
            (
                "wheel 0 -40",
                Do::Scroll(Place::Pointer, Vec2::new(0.0, -40.0)),
            ),
            ("pinch 0.2", Do::Pinch(0.2)),
            ("tool shape", Do::Act(Action::SetTool(Tool::AddShape))),
            (
                "select a b",
                Do::Select(vec!["a".to_owned(), "b".to_owned()]),
            ),
            ("act zoom-to-fit", Do::Act(Action::ZoomToFit)),
            ("camera fit", Do::Act(Action::ZoomToFit)),
            (
                "camera 40,-20,0.5",
                Do::Act(Action::SetCamera(Camera::new(Vec2::new(40.0, -20.0), 0.5))),
            ),
            ("wait 250", Do::Wait(250)),
            ("control shape.color", Do::Click(named("shape.color"), 1)),
            ("hover-control tool.draw", Do::Move(named("tool.draw"))),
            ("press-control tool.draw", Do::Press(named("tool.draw"))),
            ("sidebar on", Do::Sidebar(true)),
            ("snapshot out.png", Do::Shot(PathBuf::from("out.png"))),
            ("save out.canvas", Do::Save(PathBuf::from("out.canvas"))),
        ];
        for (text, expected) in cases {
            assert_eq!(parse(text, Dialect::File), Ok(expected), "{text}");
        }
        for (text, names) in [
            ("panels off", "`panels`"),
            ("key cmd+é", "`é`"),
            ("frobnicate 1", "not a step"),
        ] {
            let error = parse(text, Dialect::File).unwrap_err();
            assert!(error.contains(names), "{text}: {error}");
        }
    }

    #[test]
    fn the_window_s_own_steps_and_the_two_readings_of_key() {
        let at = Place::At(Vec2::new(10.0, 20.0));
        let cases = [
            ("release 10 20", Do::Release(at.clone())),
            ("scroll 10 20 0 -40", Do::Scroll(at, Vec2::new(0.0, -40.0))),
            ("keycode 51", key(51, 0, "\u{7f}", "\u{7f}")),
            ("keycode 0", key(0, 0, "", "")),
            ("shift-key 0 A", key(0, SHIFT, "A", "A")),
            (
                "paste-image /tmp/a.png",
                Do::PasteImage(PathBuf::from("/tmp/a.png")),
            ),
            ("cmd-key 6 z", key(6, COMMAND, "z", "z")),
            ("keys shift+cmd 6 z", key(6, SHIFT | COMMAND, "z", "z")),
            (
                "drop /a.png /b.png 300 400",
                Do::Drop(
                    vec![PathBuf::from("/a.png"), PathBuf::from("/b.png")],
                    Some(Vec2::new(300.0, 400.0)),
                ),
            ),
            (
                "drop /a.png nowhere",
                Do::Drop(vec![PathBuf::from("/a.png")], None),
            ),
            ("resize 1200 800", Do::Resize(Vec2::new(1200.0, 800.0))),
            ("pan -6 0 2000", Do::Pan(Vec2::new(-6.0, 0.0), 2000)),
            (
                "menu-choose Edit > Undo",
                Do::MenuChoose("Edit > Undo".to_owned()),
            ),
            (
                "choose first-run.new",
                Do::Choose("first-run.new".to_owned()),
            ),
            ("shot /tmp/a.png", Do::Shot(PathBuf::from("/tmp/a.png"))),
            ("quit", Do::Quit),
        ];
        for dialect in [Dialect::Inline, Dialect::File] {
            for (text, expected) in cases.clone() {
                assert_eq!(parse(text, dialect), Ok(expected), "{text}");
            }
        }
        // `key` with a number is a code only where it always was.
        assert_eq!(
            parse("key 51", Dialect::Inline),
            Ok(key(51, 0, "\u{7f}", "\u{7f}"))
        );
        assert_eq!(
            parse("key cmd+z", Dialect::Inline),
            Ok(key(6, COMMAND, "z", "z"))
        );
        assert!(parse("key 51", Dialect::File).is_err());
        assert!(parse("drop 300 400", Dialect::File).is_err());
    }

    #[test]
    fn only_command_c_x_and_v_are_clipboard_keys() {
        for (text, clipboard) in [
            ("key cmd+c", true),
            ("key cmd+x", true),
            ("key cmd+shift+v", true),
            ("key cmd+z", false),
            ("key c", false),
            ("type cxv", false),
        ] {
            let step = parse(text, Dialect::File).unwrap();
            assert_eq!(is_clipboard_key(&step), clipboard, "{text}");
        }
    }
}
