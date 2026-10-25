//! Keyboard bindings: one table of `(chord, context, action)`, and the
//! routing of a key through it.
//!
//! A key that matches a binding runs its action and goes no further. Any
//! other key goes to the entered page, if there is one. Escape is bound
//! everywhere, so it always cancels and is never forwarded.
//!
//! An entered page has every key but the ones [`kept_from_page`] lists, as
//! a browser's content has every key its chrome does not keep.

mod route;
mod shell;

pub(crate) use self::route::{binding_for, on_key, page_edit, page_edit_for};
pub use self::shell::{SHELL_KEYS, ShellCommand, kept_from_page};

use specular_doc::{BrushType, ShapeKind};

use crate::PageStep;
use crate::{
    Action, App, Chord, Focus, Format, Key, SidebarAction, Tool, ToolDefaultPatch, grid, page_state,
};

/// How far an arrow key moves the selection, in canvas units. Shift moves it
/// a grid step instead.
const NUDGE_STEP: f64 = 5.0;

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
    pub(crate) fn holds(self, app: &App) -> bool {
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
    pub(crate) action: Action,
    /// Whether holding the key down fires it again on each auto-repeat.
    pub(crate) repeats: bool,
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
    // How a page is shown. Command+Option keeps them clear of a browser's
    // own keys, which a page expects to get.
    once(
        Chord::char('f').cmd().alt(),
        Context::PageTarget,
        Action::StepPage(PageStep::Frame),
    ),
    once(
        Chord::char('f').cmd().alt().shift(),
        Context::PageTarget,
        Action::StepPage(PageStep::FrameColor),
    ),
    once(
        Chord::char('r').cmd().alt(),
        Context::PageTarget,
        Action::StepPage(PageStep::Rotate),
    ),
    once(
        Chord::char('l').cmd().alt(),
        Context::PageTarget,
        Action::StepPage(PageStep::Scheme),
    ),
    // A browser's tab keys. They work inside an entered page, which would
    // otherwise be sent them. While text is edited the arrows are the
    // editor's.
    once(Chord::char('t').cmd(), Context::Always, Action::NewPageTab),
    // Only ever a tab's key: on the Canvas tab it does nothing, and the
    // window stays.
    once(
        Chord::char('w').cmd(),
        Context::Always,
        Action::CloseTab(None),
    ),
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
