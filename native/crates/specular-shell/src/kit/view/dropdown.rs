//! A model [`Dropdown`] as a Kit popover over a button, and [`Choices`],
//! the same sections shown in place.
//!
//! A list stays open across frames, so its content is looked up in the
//! models each frame by the dropdown's name and never kept.

mod list;

use self::list::{Dismiss, sections};

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex};
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, Window,
    div, px,
};
use specular_interact::{
    Choices, Control, ControlId, ControlsModel, Dropdown, DropdownSection, Face, Icon,
    ToolbarModel, ToolbarSection,
};

use super::controls::{CONTROL, element_id, face};
use super::glyphs::{glyph, ink};
use super::named::mark;
use crate::{canvas, theme};

thread_local! {
    /// The dropdowns whose lists are open, by name.
    static OPEN: RefCell<HashSet<ControlId>> = RefCell::new(HashSet::new());
}

/// Whether the keys are with an open list: a choice in it that leaves it
/// open keeps them there, so Escape closes the list and does nothing else.
/// A list that went with the controls around it without closing has taken
/// its focus with it.
pub(super) fn list_has_keys(window: &Window, cx: &App) -> bool {
    OPEN.with(|open| !open.borrow().is_empty()) && window.focused(cx).is_some()
}

/// The dropdown named `id` as the models have it now. A list stays open
/// across frames, so it is looked up each time and never kept.
fn find_dropdown(id: &ControlId) -> Option<Dropdown> {
    fn among(controls: &[Control], id: &ControlId) -> Option<Dropdown> {
        controls.iter().find_map(|control| match control {
            Control::Dropdown(dropdown) if dropdown.id == *id => Some(dropdown.clone()),
            Control::Dropdown(dropdown) => {
                dropdown.content.iter().find_map(|section| match section {
                    DropdownSection::Controls(controls) => among(controls, id),
                    DropdownSection::Options { .. } => None,
                })
            }
            Control::Choices(choices) => choices.content.iter().find_map(|section| match section {
                DropdownSection::Controls(controls) => among(controls, id),
                DropdownSection::Options { .. } => None,
            }),
            Control::Button(_)
            | Control::Toggle(_)
            | Control::Swatches(_)
            | Control::Stepper(_)
            | Control::Field(_)
            | Control::Separator => None,
        })
    }
    fn in_toolbar(toolbar: &ToolbarModel, id: &ControlId) -> Option<Dropdown> {
        toolbar.sections.iter().find_map(|section| match section {
            ToolbarSection::Zoom(dropdown) if dropdown.id == *id => Some(dropdown.clone()),
            ToolbarSection::Zoom(_) | ToolbarSection::Tools(_) => None,
        })
    }
    let models = canvas::models()?;
    in_toolbar(&models.toolbar, id).or_else(|| {
        let shown: &ControlsModel = models.dock.as_ref().or(models.tool_options.as_ref())?;
        among(&shown.controls, id)
    })
}

/// A dropdown: a trigger showing the current value, and its sections in a
/// Kit popover. It has no tooltip, which would sit over the open list.
pub(super) fn dropdown(model: &Dropdown) -> AnyElement {
    let trigger = Button::new(element_id(&model.id.child("trigger")))
        .ghost()
        .xsmall()
        .h(px(CONTROL))
        .dropdown_caret(true)
        .child(face(&model.summary, false));
    opens(model, trigger)
}

/// A dropdown as a row of the list it is in: its value at the start and a
/// chevron at the far end, with its own list opening beside it.
pub(super) fn dropdown_row(model: &Dropdown) -> AnyElement {
    let trigger = Button::new(element_id(&model.id.child("trigger")))
        .ghost()
        .xsmall()
        .h(px(CONTROL))
        .w_full()
        .px_2()
        .child(
            h_flex()
                .flex_1()
                .items_center()
                .justify_between()
                .child(face(&model.summary, false))
                .child(glyph(
                    Icon::ChevronRight,
                    ink(theme::text_muted()),
                    None,
                    false,
                    12.0,
                )),
        );
    div()
        .w_full()
        .child(opens(model, trigger))
        .into_any_element()
}

/// The number a dropdown steps, for one whose list ends in a stepper.
fn stepped(model: &Dropdown) -> Option<Face> {
    model.content.iter().find_map(|section| match section {
        DropdownSection::Controls(controls) => controls.iter().find_map(|control| match control {
            Control::Stepper(stepper) => Some(Face::text(stepper.value.clone())),
            Control::Button(_)
            | Control::Toggle(_)
            | Control::Swatches(_)
            | Control::Dropdown(_)
            | Control::Field(_)
            | Control::Choices(_)
            | Control::Separator => None,
        }),
        DropdownSection::Options { .. } => None,
    })
}

/// What a dropdown's trigger shows where there is room for a glyph or a
/// few letters and no more: its glyph alone, a sample of its typeface, or
/// the number it steps. The sizes are named at length, so theirs is the
/// size itself.
fn compact_face(model: &Dropdown) -> Face {
    let summary = &model.summary;
    if summary.icon.is_some() {
        return Face {
            text: None,
            ..summary.clone()
        };
    }
    if let Some(font) = summary.font {
        return Face::text("Aa").in_font(font);
    }
    stepped(model).unwrap_or_else(|| summary.clone())
}

/// A dropdown in a narrow column: its trigger fills its cell and shows
/// [`compact_face`], with no caret.
pub(super) fn dropdown_compact(model: &Dropdown) -> AnyElement {
    let trigger = Button::new(element_id(&model.id.child("trigger")))
        .ghost()
        .xsmall()
        .h(px(CONTROL))
        .w_full()
        .px_0()
        .child(face(&compact_face(model), false))
        .child(crate::tip::over(model.id.as_str(), model.label.to_string()));
    opens(model, trigger)
}

/// `trigger` opening the sections of `model` in a Kit popover.
fn opens(model: &Dropdown, trigger: Button) -> AnyElement {
    let id = model.id.clone();
    let trigger = trigger
        // The trigger is what the model's name for the dropdown opens.
        .child(mark(&model.id));
    let named = model.id.clone();
    Popover::new(element_id(&model.id))
        // The room a Kit menu leaves around its rows, so every list that
        // opens has the same edge.
        .p_1()
        .trigger(trigger)
        .on_open_change(move |opened, _, _| {
            OPEN.with(|open| {
                let mut open = open.borrow_mut();
                if *opened {
                    open.insert(named.clone());
                } else {
                    open.remove(&named);
                }
            });
        })
        .content(move |_, window, cx| {
            let popover = cx.entity();
            let dismiss: Dismiss = Rc::new(move |window, cx| {
                popover.update(cx, |state, cx| state.dismiss(window, cx));
            });
            let content = find_dropdown(&id).map(|dropdown| dropdown.content);
            div()
                .text_color(cx.theme().popover_foreground)
                .child(sections(&content.unwrap_or_default(), &dismiss, window, cx))
        })
        .into_any_element()
}

/// A dropdown's list shown in place, with no trigger. A choice leaves the
/// list where it is.
pub(super) fn listed(model: &Dropdown, window: &mut Window, cx: &mut App) -> AnyElement {
    let stay: Dismiss = Rc::new(|_, _| {});
    div()
        .id(element_id(&model.id))
        .child(sections(&model.content, &stay, window, cx))
        .child(mark(&model.id))
        .into_any_element()
}

/// A list shown in place: its sections one under another, as an open
/// dropdown has them. A choice leaves the list where it is.
pub(super) fn choices(model: &Choices, window: &mut Window, cx: &mut App) -> AnyElement {
    let stay: Dismiss = Rc::new(|_, _| {});
    div()
        .id(element_id(&model.id))
        .child(sections(&model.content, &stay, window, cx))
        .into_any_element()
}
