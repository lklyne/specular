//! The keys the window keeps for itself, and the chords a page never gets.

use super::{BINDINGS, Context};
use crate::Chord;

/// What the window does itself, outside `update`: the keys of the app,
/// File and Window menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellCommand {
    /// Quits.
    Quit,
    /// Opens another space.
    OpenSpace,
    /// Opens a canvas file.
    OpenCanvas,
    /// Writes every canvas now.
    Save,
    /// Opens the settings.
    Settings,
    /// Hides the app.
    Hide,
    /// Hides every other app.
    HideOthers,
    /// Minimizes the window.
    Minimize,
    /// Opens the command palette.
    CommandPalette,
}

impl ShellCommand {
    /// The command's name in the command palette. The palette itself has no
    /// entry: it is already open when a command is picked.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Quit => "Quit",
            Self::OpenSpace => "Open space",
            Self::OpenCanvas => "Open",
            Self::Save => "Save",
            Self::Settings => "Settings",
            Self::Hide => "Hide",
            Self::HideOthers => "Hide others",
            Self::Minimize => "Minimize",
            Self::CommandPalette => "Command palette",
        }
    }
}

/// The window's own keys. They fire wherever the keys are, an entered page
/// and a text field included, so none may be a chord of [`BINDINGS`].
pub const SHELL_KEYS: &[(Chord, ShellCommand)] = &[
    (Chord::char('q').cmd(), ShellCommand::Quit),
    (Chord::char('o').cmd().shift(), ShellCommand::OpenSpace),
    (Chord::char('o').cmd(), ShellCommand::OpenCanvas),
    (Chord::char('s').cmd(), ShellCommand::Save),
    (Chord::char(',').cmd(), ShellCommand::Settings),
    (Chord::char('h').cmd(), ShellCommand::Hide),
    (Chord::char('h').cmd().alt(), ShellCommand::HideOthers),
    (Chord::char('m').cmd(), ShellCommand::Minimize),
    (Chord::char('k').cmd(), ShellCommand::CommandPalette),
];

/// Every chord the app keeps for itself while a page is entered, as a
/// browser's chrome keeps a few from its content. Any other key is the
/// page's.
pub fn kept_from_page() -> impl Iterator<Item = Chord> {
    let bound = BINDINGS
        .iter()
        .filter(|binding| {
            matches!(
                binding.context,
                Context::Always | Context::EnteredPage | Context::PageTarget | Context::FrameTarget
            )
        })
        .map(|binding| binding.chord);
    bound.chain(SHELL_KEYS.iter().map(|&(chord, _)| chord))
}
