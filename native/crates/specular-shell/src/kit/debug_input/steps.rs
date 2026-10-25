//! A script's steps as the things the window runner does.
//!
//! The script is read by `specular_shell::script`, the same module the
//! headless runner reads it with, so a scenario file runs here as written.
//! Each step is lowered here to what the window does for it: a pointer or
//! key event posted to the application, an `Action` through `update`, or a
//! check read from the app. `panels` has no meaning in this window, which
//! always draws the Kit's own chrome, so it is skipped.

use std::path::PathBuf;

use glam::Vec2;
use specular_interact::{Action, Event, PageNotice};
use specular_shell::script::{CameraArg, CanvasStep, Expect, PanelStep, Step, WindowStep};

use super::us_keys::{COMMAND, flags_of, key_of, key_step};

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
    /// The window's bottom-right corner dragged by this much over this many
    /// milliseconds, a move every refresh: a live resize.
    ResizeDrag(Vec2, u64),
    /// The pointer pressed where it is, dragged by this much over this many
    /// milliseconds with a move every refresh, and released this many
    /// milliseconds after the last move.
    Swipe(Vec2, u64, u64),
    /// A trackpad pinch where the pointer is.
    Pinch(f32),
    Act(Action),
    /// An event as the shell or a page would send it.
    Send(Event),
    /// The shell's or the agent's side of a panel, read against the app
    /// when it is its turn.
    Panel(PanelStep),
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
    /// The window moved so its frame's corner is here, in screen points.
    MoveWindow(Vec2),
    /// Into full screen, or back out.
    FullScreen,
    MenuDump(PathBuf),
    MenuChoose(String),
    /// The app's state as text, in the `expect` words, and the window's.
    State(PathBuf),
    /// What the window exposes to assistive apps, as text.
    Accessibility(PathBuf),
    /// The window as the window server composites it.
    Shot(PathBuf),
    /// The `.canvas` text an autosave would write.
    Save(PathBuf),
    /// A fact about the app or the saved canvas, which must hold.
    Expect(Expect),
    /// The canvas remembered under a name, for `same-as mark NAME`.
    Mark(String),
    /// The canvas written and read back.
    Reopen,
    /// Undone until nothing is left, and checked against the start but
    /// for these paths.
    UndoToStart(Vec<String>),
    Quit,
}

/// A script step as what the window does for it.
pub(super) fn lower(step: Step) -> Result<Do, String> {
    Ok(match step {
        Step::Move(at) => Do::Move(Place::At(at)),
        Step::Press(at) => Do::Press(Place::At(at)),
        Step::DragTo(at) => Do::DragTo(at),
        Step::Release => Do::Release(Place::Pointer),
        Step::ReleaseAt(at) => Do::Release(Place::At(at)),
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
        Step::ScrollAt(at, delta) => Do::Scroll(Place::At(at), delta),
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
        Step::Sidebar(shown) => Do::Sidebar(shown),
        Step::Motion(on) => Do::Send(Event::Motion(on)),
        Step::ReduceMotion(on) => Do::Send(Event::ReduceMotion(on)),
        Step::PageOpensTab {
            page,
            url,
            foreground,
        } => Do::Send(Event::Page {
            page: page.as_str().into(),
            notice: PageNotice::OpenTab { url, foreground },
        }),
        Step::Snapshot(path) => Do::Shot(path),
        Step::Save(path) => Do::Save(path),
        Step::State(path) => Do::State(path),
        Step::Expect(expect) => Do::Expect(expect),
        Step::Mark(name) => Do::Mark(name),
        Step::Reopen => Do::Reopen,
        Step::UndoToStart(except) => Do::UndoToStart(except),
        Step::Panel(step) => Do::Panel(step),
        Step::Window(step) => window(step),
        Step::Canvas(CanvasStep::Menu(item)) => Do::MenuChoose(item),
        Step::Canvas(CanvasStep::Send(event)) => Do::Send(event),
    })
}

fn window(step: WindowStep) -> Do {
    match step {
        WindowStep::KeyCode { code, held, chars } => {
            let characters = chars
                .or_else(|| named_characters(code).map(str::to_owned))
                .unwrap_or_default();
            Do::Key {
                code,
                flags: flags_of(held),
                plain: characters.clone(),
                characters,
            }
        }
        WindowStep::Pan(by, ms) => Do::Pan(by, ms),
        WindowStep::ResizeDrag(by, ms) => Do::ResizeDrag(by, ms),
        WindowStep::Swipe(by, ms, late) => Do::Swipe(by, ms, late),
        WindowStep::Drop(paths, at) => Do::Drop(paths, at),
        WindowStep::PasteImage(path) => Do::PasteImage(path),
        WindowStep::Choose(name) => Do::Choose(name),
        WindowStep::Resize(size) => Do::Resize(size),
        WindowStep::MoveWindow(to) => Do::MoveWindow(to),
        WindowStep::FullScreen => Do::FullScreen,
        WindowStep::MenuDump(path) => Do::MenuDump(path),
        WindowStep::MenuChoose(item) => Do::MenuChoose(item),
        WindowStep::Accessibility(path) => Do::Accessibility(path),
        WindowStep::Quit => Do::Quit,
    }
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

/// `SPECULAR_SHELL_SCRIPT`'s `;`-separated steps as script lines. There a
/// `key` followed by a number is a key code, as it was before `key cmd+z`
/// existed.
pub(super) fn inline_lines(script: &str) -> String {
    let steps = script
        .split(';')
        .map(str::trim)
        .filter(|step| !step.is_empty());
    let lines: Vec<String> = steps
        .map(|step| match step.strip_prefix("key ") {
            Some(rest) if rest.split_whitespace().next().is_some_and(is_code) => {
                format!("keycode {rest}")
            }
            _ => step.to_owned(),
        })
        .collect();
    lines.join("\n")
}

fn is_code(word: &str) -> bool {
    word.parse::<u16>().is_ok()
}

/// Whether `step` copies, cuts or pastes by its key.
pub(super) fn is_clipboard_key(step: &Do) -> bool {
    let keys = ['c', 'x', 'v'].map(|key| key_of(key).map(|(code, _)| code));
    matches!(step, Do::Key { code, flags, .. }
        if flags & COMMAND != 0 && keys.contains(&Some(*code)))
}
