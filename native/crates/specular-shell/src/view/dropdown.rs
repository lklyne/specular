//! A model [`Dropdown`] as a Kit popover over a button, and [`Choices`],
//! the same sections shown in place.
//!
//! A list stays open across frames, so its content is looked up in the
//! models each frame by the dropdown's name and never kept.

use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use specular_interact::{
    Action, Choices, Chord, Control, ControlId, Dropdown, DropdownOption, DropdownSection, Icon,
    OptionLayout, PopupModel, Tool, ToolbarModel, ToolbarSection,
};

use super::controls::{CONTROL, control, element_id, face};
use super::glyphs::{glyph, ink};
use super::named::mark;
use super::run;
use crate::canvas;
use crate::theme;

/// How a list's rows are drawn, as the built-in panels have it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rows {
    /// Words marked with a check: every row in the full text colour, and
    /// the current one with a check at its far end.
    Menu,
    /// Rows with small text or a key at the far end: the current one
    /// filled, the others muted.
    Presets,
}

/// Which way `content` is drawn: a list with any far-end text or key is
/// presets, and any other a menu.
fn rows_of(content: &[DropdownSection]) -> Rows {
    let detailed = content.iter().any(|section| match section {
        DropdownSection::Options { options, .. } => {
            (options.iter()).any(|option| option.trailing.is_some() || option.chord.is_some())
        }
        DropdownSection::Controls(_) => false,
    });
    if detailed { Rows::Presets } else { Rows::Menu }
}

/// One choice of a dropdown: a row of a list, with its small text or key at
/// the far end, or a cell of a row or grid of glyphs.
fn option(model: &DropdownOption, list: Option<Rows>, dismiss: Dismiss) -> AnyElement {
    let action = model.action.clone();
    let wide = list.is_some();
    let menu = list == Some(Rows::Menu);
    let height = match (list, model.face.font) {
        (None, _) => 32.0,
        (Some(_), Some(_)) => 28.0,
        (Some(_), None) => CONTROL,
    };
    let trailing = (model.trailing.as_ref()).map(|text| SharedString::from(text.to_string()));
    let keys = model.chord.map(Chord::text);
    div()
        .id(element_id(&model.id))
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .h(px(height))
        .when(wide, |this| this.w_full().px_2())
        .when(!wide, |this| this.w(px(32.0)).justify_center())
        .rounded(px(6.0))
        .when(model.selected && !menu, |this| {
            this.bg(theme::solid(theme::control_on()))
        })
        .when(!model.enabled, |this| this.opacity(0.4))
        .when(model.enabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(theme::solid(theme::control_hover())))
                .on_click(move |_, window, cx| {
                    run(&action, window, cx);
                    dismiss(window, cx);
                })
        })
        .child(face(&model.face, model.selected || menu))
        .child(mark(&model.id))
        .when(model.selected && menu, |this| {
            this.child(glyph(Icon::Check, ink(theme::text()), None, false, 12.0))
        })
        .when_some(trailing, |this, text| {
            this.child(
                div()
                    .text_color(theme::tinted(theme::text_muted()))
                    .child(text),
            )
        })
        .when_some(keys, |this, keys| {
            this.child(
                div()
                    .px_1p5()
                    .py_0p5()
                    .rounded(px(4.0))
                    .bg(theme::solid(theme::control_on()))
                    .text_color(theme::solid(theme::key_text()))
                    .child(SharedString::from(keys)),
            )
        })
        .into_any_element()
}

/// Closes the list a chosen option was in.
type Dismiss = Rc<dyn Fn(&mut Window, &mut App)>;

fn section(
    model: &DropdownSection,
    rows: Rows,
    dismiss: &Dismiss,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    match model {
        DropdownSection::Options { layout, options } => match layout {
            OptionLayout::List => v_flex()
                .gap_0p5()
                .min_w(px(140.0))
                .children((options.iter()).map(|one| option(one, Some(rows), dismiss.clone())))
                .into_any_element(),
            OptionLayout::Row => h_flex()
                .gap_0p5()
                .children(options.iter().map(|one| option(one, None, dismiss.clone())))
                .into_any_element(),
            OptionLayout::Grid { columns } => {
                let width = f32::from(*columns) * 32.0 + f32::from(columns.saturating_sub(1)) * 2.0;
                h_flex()
                    .flex_wrap()
                    .gap(px(2.0))
                    .w(px(width))
                    .children(options.iter().map(|one| option(one, None, dismiss.clone())))
                    .into_any_element()
            }
        },
        DropdownSection::Controls(controls) => {
            let controls: Vec<AnyElement> = (controls.iter())
                .map(|model| control(model, window, cx))
                .collect();
            h_flex()
                .gap_1()
                .items_center()
                .children(controls)
                .into_any_element()
        }
    }
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
        let dock: &PopupModel = models.dock.as_ref()?;
        among(&dock.controls, id)
    })
}

/// Sections one under another with a line between them, as an open list
/// has them.
fn sections(
    content: &[DropdownSection],
    dismiss: &Dismiss,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let last = content.len().saturating_sub(1);
    let rows = rows_of(content);
    let blocks: Vec<AnyElement> = (content.iter().enumerate())
        .map(|(index, one)| {
            v_flex()
                .gap_1()
                .child(section(one, rows, dismiss, window, cx))
                .when(index < last, |this| this.child(Separator::horizontal()))
                .into_any_element()
        })
        .collect();
    v_flex()
        .gap_1()
        .text_size(px(12.0))
        .children(blocks)
        .into_any_element()
}

/// A dropdown: a trigger showing the current value, and its sections in a
/// Kit popover. It has no tooltip, which would sit over the open list.
/// `toolbar` is the zoom readout, as tall as a tool button. A tool's popup
/// hangs where a list of the toolbar opens, so opening one puts the tool
/// down.
pub(super) fn dropdown(model: &Dropdown, toolbar: bool) -> AnyElement {
    let id = model.id.clone();
    let trigger = Button::new(element_id(&model.id.child("trigger")))
        .ghost()
        .xsmall()
        .h(px(if toolbar { 28.0 } else { CONTROL }))
        .dropdown_caret(true)
        .child(face(&model.summary, false))
        // The trigger is what the model's name for the dropdown opens.
        .child(mark(&model.id));
    Popover::new(element_id(&model.id))
        .trigger(trigger)
        .on_open_change(move |open, window, cx| {
            let in_hand = canvas::with(|canvas| canvas.runtime.app().session().tool);
            if *open && toolbar && in_hand.is_some_and(|tool| tool != Tool::Select) {
                run(&Action::SetTool(Tool::Select), window, cx);
            }
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

/// A list shown in place: its sections one under another, as an open
/// dropdown has them. A choice leaves the list where it is.
pub(super) fn choices(model: &Choices, window: &mut Window, cx: &mut App) -> AnyElement {
    let stay: Dismiss = Rc::new(|_, _| {});
    div()
        .id(element_id(&model.id))
        .child(sections(&model.content, &stay, window, cx))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use specular_interact::{Face, Key};

    use super::*;

    fn option(trailing: Option<&'static str>, chord: Option<Chord>) -> DropdownOption {
        DropdownOption {
            id: ControlId::new("option"),
            label: "Option".into(),
            face: Face::text("Option"),
            trailing: trailing.map(Into::into),
            chord,
            selected: false,
            enabled: true,
            action: Action::ZoomReset,
        }
    }

    #[test]
    fn a_list_with_far_end_text_or_a_key_is_drawn_as_presets() {
        let list = |options| DropdownSection::Options {
            layout: OptionLayout::List,
            options,
        };
        let cases = [
            (vec![list(vec![option(None, None)])], Rows::Menu),
            (
                vec![list(vec![option(Some("375×667"), None)])],
                Rows::Presets,
            ),
            (
                vec![list(vec![option(None, Some(Chord::key(Key::Enter)))])],
                Rows::Presets,
            ),
            (
                vec![
                    list(vec![option(None, None)]),
                    DropdownSection::Controls(vec![]),
                ],
                Rows::Menu,
            ),
        ];
        for (content, expected) in cases {
            assert_eq!(rows_of(&content), expected);
        }
    }
}
