//! Keyboard bindings: one table of `(chord, context, action)`, and the
//! routing of a key through it.
//!
//! A key that matches a binding runs its action and goes no further. Any
//! other key goes to the entered page, if there is one. Escape is bound
//! everywhere, so it always cancels and is never forwarded.

use specular_core::PointerEventKind;
use specular_doc::{BrushType, ShapeKind};

use crate::update::run_action;
use crate::{
    Action, App, Effect, Focus, Format, Key, KeyInput, PointerInput, SidebarAction, Tool,
    ToolDefaultPatch, edit, gesture, grid, page_input, page_state,
};

/// How far an arrow key moves the selection, in canvas units. Shift moves it
/// a grid step instead.
const NUDGE_STEP: f64 = 5.0;

/// A key with the modifiers that must be held, and no others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    /// The physical key.
    pub key: Key,
    /// Command or Control.
    pub cmd: bool,
    /// Shift.
    pub shift: bool,
    /// Option.
    pub alt: bool,
}

impl Chord {
    /// The chord as macOS writes it: `⇧⌘Z`.
    pub fn text(self) -> String {
        let mut text = String::new();
        for (held, sign) in [(self.alt, '⌥'), (self.shift, '⇧'), (self.cmd, '⌘')] {
            if held {
                text.push(sign);
            }
        }
        match self.key {
            Key::Char(character) => text.extend(character.to_uppercase()),
            Key::Escape => text.push('⎋'),
            Key::Enter => text.push('↩'),
            Key::Tab => text.push('⇥'),
            Key::Backspace => text.push('⌫'),
            Key::Delete => text.push('⌦'),
            Key::Space => text.push_str("Space"),
            Key::ArrowLeft => text.push('←'),
            Key::ArrowRight => text.push('→'),
            Key::ArrowUp => text.push('↑'),
            Key::ArrowDown => text.push('↓'),
            Key::Home => text.push('↖'),
            Key::End => text.push('↘'),
            Key::PageUp => text.push('⇞'),
            Key::PageDown => text.push('⇟'),
            Key::Other => {}
        }
        text
    }

    /// `key` with no modifiers.
    pub const fn key(key: Key) -> Self {
        Self {
            key,
            cmd: false,
            shift: false,
            alt: false,
        }
    }

    /// The letter, digit or punctuation key `character` with no modifiers.
    pub const fn char(character: char) -> Self {
        Self::key(Key::Char(character))
    }

    /// The same key with Command or Control held as well.
    #[must_use]
    pub const fn cmd(self) -> Self {
        Self { cmd: true, ..self }
    }

    /// The same key with Shift held as well.
    #[must_use]
    pub const fn shift(self) -> Self {
        Self {
            shift: true,
            ..self
        }
    }

    /// The same key with Option held as well.
    #[must_use]
    pub const fn alt(self) -> Self {
        Self { alt: true, ..self }
    }

    /// The chord `input` is, whether it is a press or a release.
    ///
    /// Escape is Escape whatever is held with it, so it cancels a drag that
    /// has Shift or Option down.
    pub fn of(input: &KeyInput) -> Self {
        if input.key == Key::Escape {
            return Self::key(Key::Escape);
        }
        let modifiers = input.modifiers;
        Self {
            key: input.key,
            cmd: modifiers.meta || modifiers.control,
            shift: modifiers.shift,
            alt: modifiers.alt,
        }
    }
}

/// Where a binding fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Context {
    /// Everywhere, an entered page included.
    Always,
    /// While keys go to the canvas and no text is being edited. Plain
    /// letters live here, so typing never switches tool.
    Canvas,
    /// While keys go to the canvas, text being edited or not.
    CanvasOrEditing,
    /// While text is being edited.
    Editing,
    /// While a page is entered. Its keys reach the page unless a row here
    /// takes them first.
    EnteredPage,
    /// While a page is entered, or is the whole selection with keys going
    /// to the canvas and no text being edited.
    PageTarget,
}

impl Context {
    /// Whether there is a state of the app in which both contexts hold. Two
    /// rows of the table may share a key only where this is false.
    pub fn overlaps(self, other: Self) -> bool {
        // Keys go either to the canvas or to an entered page, never both,
        // and text is either being edited or not.
        let apart = |one, other| {
            matches!(
                (one, other),
                (Self::Canvas | Self::CanvasOrEditing, Self::EnteredPage)
                    | (Self::Canvas, Self::Editing)
            )
        };
        !(apart(self, other) || apart(other, self))
    }

    /// Whether a binding with this context fires in `app` as it is now.
    pub fn holds(self, app: &App) -> bool {
        let session = &app.session;
        match self {
            Self::Always => true,
            Self::Canvas => session.focus == Focus::Canvas && session.editing.is_none(),
            Self::CanvasOrEditing => session.focus == Focus::Canvas,
            Self::Editing => session.editing.is_some(),
            Self::EnteredPage => matches!(session.focus, Focus::Page(_)),
            Self::PageTarget => {
                page_state::target(app).is_some()
                    && (Self::EnteredPage.holds(app) || Self::Canvas.holds(app))
            }
        }
    }
}

/// One row of the binding table.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    /// The key and modifiers.
    pub chord: Chord,
    /// Where it fires.
    pub context: Context,
    /// What it does.
    pub action: Action,
    /// Whether holding the key down fires it again on each auto-repeat.
    pub repeats: bool,
}

const fn once(chord: Chord, context: Context, action: Action) -> Binding {
    Binding {
        chord,
        context,
        action,
        repeats: false,
    }
}

const fn held(chord: Chord, context: Context, action: Action) -> Binding {
    Binding {
        chord,
        context,
        action,
        repeats: true,
    }
}

const fn tool(character: char, tool: Tool) -> Binding {
    once(
        Chord::char(character),
        Context::Canvas,
        Action::SetTool(tool),
    )
}

const fn variant(chord: Chord, patch: ToolDefaultPatch) -> Binding {
    once(chord, Context::Canvas, Action::SetToolVariant(patch))
}

const fn format(chord: Chord, format: Format) -> Binding {
    once(chord, Context::Editing, Action::Format(format))
}

/// Command+Option+digit: a heading of that level, and 0 for body text.
const fn heading(level: u8) -> Binding {
    let digit = (b'0' + level) as char;
    format(Chord::char(digit).cmd().alt(), Format::Heading(level))
}

/// An arrow key's nudge: `dx` and `dy` are the direction, and `chord` holding
/// Shift makes the step a grid step.
const fn nudge(chord: Chord, dx: f64, dy: f64) -> Binding {
    let step = if chord.shift {
        grid::GRID_SIZE
    } else {
        NUDGE_STEP
    };
    held(
        chord,
        Context::Canvas,
        Action::Nudge {
            dx: dx * step,
            dy: dy * step,
        },
    )
}

/// Every key binding. The first row whose chord matches and whose context
/// holds wins.
///
/// A tool's key does nothing while that tool is active; Escape is the only
/// key back to select. A variant key (Shift+R, Shift+M) arms the tool and
/// writes the variant to the tool defaults (ADR 0009).
///
/// The formatting rows are the Electron editor's keys. It has none for a
/// numbered list, a task list or a heading: those take the digits beside
/// its Command+Shift+8, and Command+Option+digit.
pub const BINDINGS: &[Binding] = &[
    tool('v', Tool::Select),
    tool('p', Tool::AddPage),
    tool('t', Tool::AddText),
    tool('s', Tool::AddSticky),
    variant(
        Chord::char('r'),
        ToolDefaultPatch::ShapeKind(ShapeKind::Rectangle),
    ),
    variant(
        Chord::char('o'),
        ToolDefaultPatch::ShapeKind(ShapeKind::Ellipse),
    ),
    variant(
        Chord::char('r').shift(),
        ToolDefaultPatch::ShapeKind(ShapeKind::Diamond),
    ),
    tool('c', Tool::Comment),
    tool('i', Tool::Inspect),
    variant(Chord::char('m'), ToolDefaultPatch::Brush(BrushType::Pen)),
    variant(
        Chord::char('m').shift(),
        ToolDefaultPatch::Brush(BrushType::Highlight),
    ),
    held(
        Chord::char('z').cmd(),
        Context::CanvasOrEditing,
        Action::Undo,
    ),
    held(
        Chord::char('z').cmd().shift(),
        Context::CanvasOrEditing,
        Action::Redo,
    ),
    once(Chord::char('x').cmd(), Context::Canvas, Action::Cut),
    once(Chord::char('c').cmd(), Context::Canvas, Action::Copy),
    once(
        Chord::char('v').cmd(),
        Context::CanvasOrEditing,
        Action::Paste,
    ),
    once(Chord::char('d').cmd(), Context::Canvas, Action::Duplicate),
    once(Chord::key(Key::Backspace), Context::Canvas, Action::Delete),
    once(Chord::key(Key::Delete), Context::Canvas, Action::Delete),
    once(Chord::char('a').cmd(), Context::Canvas, Action::SelectAll),
    once(
        Chord::char(']').cmd(),
        Context::Canvas,
        Action::BringForward,
    ),
    once(
        Chord::char('[').cmd(),
        Context::Canvas,
        Action::SendBackward,
    ),
    once(
        Chord::char(']').cmd().shift(),
        Context::Canvas,
        Action::BringToFront,
    ),
    once(
        Chord::char('[').cmd().shift(),
        Context::Canvas,
        Action::SendToBack,
    ),
    once(
        Chord::char('a').cmd().shift(),
        Context::Canvas,
        Action::AutoLayout,
    ),
    once(Chord::char('g').cmd(), Context::Canvas, Action::Group),
    once(
        Chord::char('g').cmd().shift(),
        Context::Canvas,
        Action::Ungroup,
    ),
    held(
        Chord::char('=').cmd(),
        Context::CanvasOrEditing,
        Action::ZoomIn,
    ),
    held(
        Chord::char('-').cmd(),
        Context::CanvasOrEditing,
        Action::ZoomOut,
    ),
    once(
        Chord::char('0').cmd(),
        Context::CanvasOrEditing,
        Action::ZoomReset,
    ),
    once(Chord::char('1').cmd(), Context::Always, Action::ZoomToFit),
    // The same key is bold while text is edited.
    once(
        Chord::char('b').cmd(),
        Context::Canvas,
        Action::Sidebar(SidebarAction::Toggle),
    ),
    nudge(Chord::key(Key::ArrowLeft), -1.0, 0.0),
    nudge(Chord::key(Key::ArrowRight), 1.0, 0.0),
    nudge(Chord::key(Key::ArrowUp), 0.0, -1.0),
    nudge(Chord::key(Key::ArrowDown), 0.0, 1.0),
    nudge(Chord::key(Key::ArrowLeft).shift(), -1.0, 0.0),
    nudge(Chord::key(Key::ArrowRight).shift(), 1.0, 0.0),
    nudge(Chord::key(Key::ArrowUp).shift(), 0.0, -1.0),
    nudge(Chord::key(Key::ArrowDown).shift(), 0.0, 1.0),
    format(Chord::char('b').cmd(), Format::Bold),
    format(Chord::char('i').cmd(), Format::Italic),
    format(Chord::char('e').cmd(), Format::Code),
    format(Chord::char('x').cmd().shift(), Format::Strike),
    format(Chord::char('8').cmd().shift(), Format::BulletList),
    format(Chord::char('7').cmd().shift(), Format::NumberedList),
    format(Chord::char('9').cmd().shift(), Format::TaskList),
    heading(0),
    heading(1),
    heading(2),
    heading(3),
    heading(4),
    heading(5),
    heading(6),
    // A browser's keys. On the canvas the bracket keys restack, so they
    // only walk the history of a page that is entered.
    once(
        Chord::char('[').cmd(),
        Context::EnteredPage,
        Action::PageBack,
    ),
    once(
        Chord::char(']').cmd(),
        Context::EnteredPage,
        Action::PageForward,
    ),
    once(
        Chord::char('r').cmd(),
        Context::PageTarget,
        Action::PageReload,
    ),
    once(
        Chord::char('.').cmd(),
        Context::PageTarget,
        Action::PageStop,
    ),
    once(
        Chord::char('l').cmd(),
        Context::PageTarget,
        Action::EditPageUrl,
    ),
    // A browser's tab keys. They work inside an entered page, which would
    // otherwise be sent them. While text is edited the arrows are the
    // editor's.
    once(Chord::char('t').cmd(), Context::Always, Action::NewPageTab),
    once(
        Chord::key(Key::ArrowRight).cmd().alt(),
        Context::Canvas,
        Action::ShowNext,
    ),
    once(
        Chord::key(Key::ArrowLeft).cmd().alt(),
        Context::Canvas,
        Action::ShowPrevious,
    ),
    once(
        Chord::key(Key::ArrowRight).cmd().alt(),
        Context::EnteredPage,
        Action::ShowNext,
    ),
    once(
        Chord::key(Key::ArrowLeft).cmd().alt(),
        Context::EnteredPage,
        Action::ShowPrevious,
    ),
    once(Chord::key(Key::Escape), Context::Always, Action::Cancel),
];

/// The binding `input`'s key and modifiers fire in `app` as it is now.
pub fn binding_for(app: &App, input: &KeyInput) -> Option<&'static Binding> {
    let chord = Chord::of(input);
    BINDINGS
        .iter()
        .find(|binding| binding.chord == chord && binding.context.holds(app))
}

pub(crate) fn on_key(app: &mut App, input: &KeyInput, effects: &mut Vec<Effect>) {
    app.session.modifiers = input.modifiers;
    // A drag reads Shift, Option and Command, so one that changes while the
    // pointer is still takes effect at once.
    if let (Some(_), Some(screen)) = (&app.session.gesture, app.session.pointer) {
        let held = PointerInput {
            kind: PointerEventKind::Move,
            screen,
            modifiers: input.modifiers,
        };
        gesture::drag(app, &held);
    }
    // The text being edited has the keys first. It leaves Escape, undo and
    // redo to the table.
    if edit::on_key(app, input, effects) {
        return;
    }
    match binding_for(app, input) {
        Some(binding) => {
            // The release of a bound key goes nowhere either.
            if input.pressed && (binding.repeats || !input.repeat) {
                run_action(app, binding.action.clone(), effects);
            }
        }
        None => match &app.session.focus {
            Focus::Page(page) => page_input::forward_key(page, input, effects),
            Focus::Canvas => {}
        },
    }
}
