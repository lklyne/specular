//! The plain data a toolbar or dock renderer draws: which controls exist
//! now, their state, and the [`Action`] each one dispatches. Nothing here
//! says how a control looks or where it sits on screen.

use std::borrow::Cow;

use specular_doc::{Color, TextFont};

use super::{ControlId, Field, Icon};
use crate::{Action, Chord};

/// Text a model carries: a literal, or a line made for this selection.
pub type Label = Cow<'static, str>;

/// A glyph, a word, or both, plus what tints them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Face {
    /// The glyph.
    pub icon: Option<Icon>,
    /// The word shown beside or instead of the glyph.
    pub text: Option<Label>,
    /// What the glyph or the swatch dot is painted in. With no icon and no
    /// text, a dot of this color, hollow when it is `None`.
    pub color: Option<Color>,
    /// The typeface the text is set in, for a list that previews itself.
    pub font: Option<TextFont>,
}

impl Face {
    /// A glyph.
    pub fn icon(icon: Icon) -> Self {
        Self {
            icon: Some(icon),
            ..Self::default()
        }
    }

    /// A word.
    pub fn text(text: impl Into<Label>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    /// A dot of `color`.
    pub fn dot(color: Option<Color>) -> Self {
        Self {
            color,
            ..Self::default()
        }
    }

    /// The same face painted in `color`.
    #[must_use]
    pub fn tinted(self, color: Option<Color>) -> Self {
        Self { color, ..self }
    }

    /// The same face set in `font`.
    #[must_use]
    pub fn in_font(self, font: TextFont) -> Self {
        Self {
            font: Some(font),
            ..self
        }
    }
}

/// A control that runs its action when pressed.
#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    /// Its name.
    pub id: ControlId,
    /// What it is called: the tooltip and the accessible name.
    pub label: Label,
    /// What it shows.
    pub face: Face,
    /// Whether it can be pressed now.
    pub enabled: bool,
    /// The key that does the same, from the binding table.
    pub chord: Option<Chord>,
    /// What pressing it does.
    pub action: Action,
}

/// A control that is on or off, and flips when pressed.
#[derive(Debug, Clone, PartialEq)]
pub struct Toggle {
    /// Its name.
    pub id: ControlId,
    /// What it is called: the tooltip and the accessible name.
    pub label: Label,
    /// What it shows.
    pub face: Face,
    /// Whether it is on.
    pub on: bool,
    /// Whether it can be pressed now.
    pub enabled: bool,
    /// The key that does the same, from the binding table.
    pub chord: Option<Chord>,
    /// What pressing it does. It names the state to go to, so a renderer
    /// never works out the opposite of `on`.
    pub action: Action,
}

/// The hues a color is drawn from, which differ between surfaces: the same
/// stored color reads muted on a sticky and saturated on a pen stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    /// Muted pastels: stickies, shapes, the highlighter.
    Soft,
    /// Saturated hues: plain text, edges, the pen, groups.
    Vivid,
}

/// How a color is used, which only changes the theme-aware neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintRole {
    /// A background.
    Fill,
    /// Marks and glyphs.
    Ink,
}

/// One choice in a row of swatches.
#[derive(Debug, Clone, PartialEq)]
pub struct Swatch {
    /// Its name.
    pub id: ControlId,
    /// Its name for people: "Red", "Transparent".
    pub label: Label,
    /// The color it sets, or `None` for the choice that paints nothing.
    pub color: Option<Color>,
    /// Whether it is the current value.
    pub selected: bool,
    /// What choosing it does.
    pub action: Action,
}

/// A row of color choices.
#[derive(Debug, Clone, PartialEq)]
pub struct Swatches {
    /// Its name.
    pub id: ControlId,
    /// The hues to draw the colors from.
    pub palette: Palette,
    /// How the colors are used.
    pub role: PaintRole,
    /// Whether the choices can be used now.
    pub enabled: bool,
    /// The choices, in palette order.
    pub options: Vec<Swatch>,
}

/// How the options of one dropdown section are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionLayout {
    /// One under another.
    List,
    /// Side by side.
    Row,
    /// Rows of `columns`.
    Grid {
        /// How many to a row.
        columns: u8,
    },
}

/// One choice in a dropdown.
#[derive(Debug, Clone, PartialEq)]
pub struct DropdownOption {
    /// Its name.
    pub id: ControlId,
    /// What it is called.
    pub label: Label,
    /// What it shows.
    pub face: Face,
    /// Small text at the far end of a row, such as a size.
    pub trailing: Option<Label>,
    /// The key that does the same.
    pub chord: Option<Chord>,
    /// Whether it is the current value.
    pub selected: bool,
    /// Whether it can be chosen now.
    pub enabled: bool,
    /// What choosing it does.
    pub action: Action,
}

/// One block of a dropdown's content. Blocks are set apart from each other.
#[derive(Debug, Clone, PartialEq)]
pub enum DropdownSection {
    /// Choices.
    Options {
        /// How they are arranged.
        layout: OptionLayout,
        /// The choices.
        options: Vec<DropdownOption>,
    },
    /// A row of controls: swatches, toggles, a stepper.
    Controls(Vec<Control>),
}

/// A control that opens more controls.
#[derive(Debug, Clone, PartialEq)]
pub struct Dropdown {
    /// Its name.
    pub id: ControlId,
    /// What it is called.
    pub label: Label,
    /// The current value, shown on the closed control.
    pub summary: Face,
    /// What opens under it.
    pub content: Vec<DropdownSection>,
}

/// Choices shown in place, the list that is a context menu, rather than
/// behind a control.
#[derive(Debug, Clone, PartialEq)]
pub struct Choices {
    /// Its name.
    pub id: ControlId,
    /// What it is called.
    pub label: Label,
    /// The blocks of choices, set apart from each other.
    pub content: Vec<DropdownSection>,
}

/// A number with a control for each direction.
#[derive(Debug, Clone, PartialEq)]
pub struct Stepper {
    /// Its name. The buttons are its children `dec` and `inc`.
    pub id: ControlId,
    /// What it is called.
    pub label: Label,
    /// The value as shown.
    pub value: Label,
    /// Lowers the value.
    pub decrement: Action,
    /// Whether the value can go lower.
    pub can_decrement: bool,
    /// Raises the value.
    pub increment: Action,
    /// Whether the value can go higher.
    pub can_increment: bool,
}

/// One control of the dock, a dropdown or a context menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    /// Runs an action.
    Button(Button),
    /// Is on or off.
    Toggle(Toggle),
    /// A row of color choices.
    Swatches(Swatches),
    /// Opens more controls.
    Dropdown(Dropdown),
    /// A number to step up and down.
    Stepper(Stepper),
    /// A line of text to type.
    Field(Field),
    /// Choices that are a whole context menu. It is the menu's only control.
    Choices(Choices),
    /// A dividing line between groups.
    Separator,
}

/// The controls and options in a model, each with the action it runs.
/// Controls that run nothing themselves, such as a dropdown, have none.
pub type Entries<'a> = Vec<(ControlId, Option<&'a Action>)>;

impl Control {
    /// Adds this control, and the ones inside it, to `out`.
    pub fn entries<'a>(&'a self, out: &mut Entries<'a>) {
        match self {
            Self::Button(button) => out.push((button.id.clone(), Some(&button.action))),
            Self::Toggle(toggle) => out.push((toggle.id.clone(), Some(&toggle.action))),
            Self::Swatches(swatches) => {
                for swatch in &swatches.options {
                    out.push((swatch.id.clone(), Some(&swatch.action)));
                }
            }
            Self::Dropdown(dropdown) => dropdown.entries(out),
            Self::Stepper(stepper) => {
                out.push((stepper.id.child("dec"), Some(&stepper.decrement)));
                out.push((stepper.id.child("inc"), Some(&stepper.increment)));
            }
            // What a field runs depends on the text, so it has no action to
            // list.
            Self::Field(field) => out.push((field.id.clone(), None)),
            Self::Choices(choices) => section_entries(&choices.content, out),
            Self::Separator => {}
        }
    }
}

impl Dropdown {
    /// Adds this dropdown, and the options and controls inside it, to `out`.
    pub fn entries<'a>(&'a self, out: &mut Entries<'a>) {
        out.push((self.id.clone(), None));
        section_entries(&self.content, out);
    }
}

fn section_entries<'a>(content: &'a [DropdownSection], out: &mut Entries<'a>) {
    for section in content {
        match section {
            DropdownSection::Options { options, .. } => {
                out.extend(options.iter().map(|o| (o.id.clone(), Some(&o.action))));
            }
            DropdownSection::Controls(controls) => {
                for control in controls {
                    control.entries(out);
                }
            }
        }
    }
}
