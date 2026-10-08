//! [`Field`]: a control whose value is a line of text a person types.
//!
//! A field holds no focus, caret or selection: a renderer keeps those. It
//! says what the field shows now and what its typed text means, as
//! [`FieldSubmit`], which is data rather than a closure so a model can be
//! compared and cloned.

use super::{Control, ControlId, DropdownSection, Label, popup_for};
use crate::{Action, App, CanvasAction, CanvasId, Property, resolve_address_input};

/// The least and greatest number a size field takes, in pixels.
const SIZE_RANGE: (f64, f64) = (1.0, 10_000.0);

/// How much room a field asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldWidth {
    /// Room for a line such as an address, and any more it is given.
    Wide,
    /// Room for a few characters, such as a number.
    Short,
}

/// What a field's text means once a person has entered it.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldSubmit {
    /// An address: a URL, or words to search for, to take the page to.
    PageUrl,
    /// A custom page width, in pixels.
    ViewportWidth,
    /// A custom page height, in pixels.
    ViewportHeight,
    /// A canvas's name, to rename it to.
    CanvasName(CanvasId),
}

impl FieldSubmit {
    /// The action that `text` asks for, or `None` when it asks for nothing:
    /// an empty address or a number that is not one. A renderer then shows
    /// the field's old value again.
    pub fn action(&self, text: &str) -> Option<Action> {
        match self {
            Self::PageUrl => resolve_address_input(text).map(Action::PageNavigate),
            Self::ViewportWidth => {
                size(text).map(|px| Action::SetProperty(Property::ViewportWidth(px)))
            }
            Self::ViewportHeight => {
                size(text).map(|px| Action::SetProperty(Property::ViewportHeight(px)))
            }
            Self::CanvasName(canvas) => {
                let name = text.trim();
                (!name.is_empty()).then(|| {
                    Action::Canvas(CanvasAction::Rename {
                        canvas: Some(canvas.clone()),
                        name: name.to_owned(),
                    })
                })
            }
        }
    }
}

/// A whole number of pixels within [`SIZE_RANGE`].
fn size(text: &str) -> Option<f64> {
    let value: f64 = text.trim().parse().ok()?;
    let rounded = value.round();
    (value.is_finite() && (SIZE_RANGE.0..=SIZE_RANGE.1).contains(&rounded)).then_some(rounded)
}

/// A line of text to type.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Its name.
    pub id: ControlId,
    /// What it is called: the tooltip and the accessible name.
    pub label: Label,
    /// A short word shown beside it, such as "W" for a width.
    pub caption: Option<Label>,
    /// What it holds now.
    pub value: String,
    /// What to show in an empty field.
    pub placeholder: Option<Label>,
    /// How much room it asks for.
    pub width: FieldWidth,
    /// What entering its text does.
    pub submit: FieldSubmit,
}

/// The field named `id` in the popup shown now, whether it sits in the
/// popup's row or inside a dropdown, or the name of a canvas in the sidebar
/// while that is shown.
pub(crate) fn field_named(app: &App, id: &ControlId) -> Option<Field> {
    if id.as_str().starts_with("sidebar.") {
        if !app.session.sidebar.shown() {
            return None;
        }
        return (crate::sidebar(app).canvases.into_iter())
            .map(|row| row.rename)
            .find(|field| field.id == *id);
    }
    let popup = popup_for(app)?;
    popup.controls.iter().find_map(|control| find(control, id))
}

fn find(control: &Control, id: &ControlId) -> Option<Field> {
    match control {
        Control::Field(field) if field.id == *id => Some(field.clone()),
        Control::Dropdown(dropdown) => dropdown.content.iter().find_map(|section| match section {
            DropdownSection::Controls(row) => row.iter().find_map(|inner| find(inner, id)),
            DropdownSection::Options { .. } => None,
        }),
        Control::Field(_)
        | Control::Button(_)
        | Control::Toggle(_)
        | Control::Swatches(_)
        | Control::Stepper(_)
        | Control::Choices(_)
        | Control::Separator => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_is_completed_and_a_blank_one_asks_for_nothing() {
        assert_eq!(
            FieldSubmit::PageUrl.action("example.com"),
            Some(Action::PageNavigate("https://example.com/".to_owned()))
        );
        assert_eq!(FieldSubmit::PageUrl.action("  "), None);
    }

    #[test]
    fn a_size_is_a_whole_number_in_range() {
        let width = |text| FieldSubmit::ViewportWidth.action(text);
        assert_eq!(
            width(" 820.4 "),
            Some(Action::SetProperty(Property::ViewportWidth(820.0)))
        );
        for text in ["", "wide", "0", "-4", "10001", "NaN", "inf"] {
            assert_eq!(width(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_canvas_name_is_trimmed_and_a_blank_one_asks_for_nothing() {
        let rename = FieldSubmit::CanvasName(CanvasId::new("c1"));
        assert_eq!(
            rename.action("  Plans "),
            Some(Action::Canvas(CanvasAction::Rename {
                canvas: Some(CanvasId::new("c1")),
                name: "Plans".to_owned(),
            }))
        );
        assert_eq!(rename.action("   "), None);
    }
}
