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
    Action, App, Effect, Focus, Key, KeyInput, PointerInput, Tool, ToolDefaultPatch, gesture, grid,
    page_input,
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
}

impl Context {
    /// Whether a binding with this context fires in `app` as it is now.
    pub fn holds(self, app: &App) -> bool {
        let session = &app.session;
        match self {
            Self::Always => true,
            Self::Canvas => session.focus == Focus::Canvas && session.editing.is_none(),
            Self::CanvasOrEditing => session.focus == Focus::Canvas,
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
    once(Chord::char('d').cmd(), Context::Canvas, Action::Duplicate),
    once(Chord::key(Key::Delete), Context::Canvas, Action::Delete),
    once(Chord::key(Key::Backspace), Context::Canvas, Action::Delete),
    nudge(Chord::key(Key::ArrowLeft), -1.0, 0.0),
    nudge(Chord::key(Key::ArrowRight), 1.0, 0.0),
    nudge(Chord::key(Key::ArrowUp), 0.0, -1.0),
    nudge(Chord::key(Key::ArrowDown), 0.0, 1.0),
    nudge(Chord::key(Key::ArrowLeft).shift(), -1.0, 0.0),
    nudge(Chord::key(Key::ArrowRight).shift(), 1.0, 0.0),
    nudge(Chord::key(Key::ArrowUp).shift(), 0.0, -1.0),
    nudge(Chord::key(Key::ArrowDown).shift(), 0.0, 1.0),
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
