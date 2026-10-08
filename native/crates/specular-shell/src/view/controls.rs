//! The one place a model control becomes a Kit component.
//!
//! | Model       | Kit                                                      |
//! |-------------|----------------------------------------------------------|
//! | `Button`    | `Button`, ghost                                          |
//! | `Toggle`    | `Toggle`                                                 |
//! | `Swatches`  | a row of round buttons (the Kit has a picker, not a row) |
//! | `Dropdown`  | `Popover` over a `Button` trigger                        |
//! | `Stepper`   | two `Button`s around the value                           |
//! | `Separator` | `Separator`                                              |
//!
//! Nothing here holds state. A control is drawn from the model each frame
//! and a click sends the model's own `Action` through `update`.

use gpui_kit::component::button::{Button, ButtonVariants as _, Toggle, ToggleVariants as _};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use specular_interact::{
    Action, Choices, Chord, Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face,
    Field, OptionLayout, PaintRole, Palette, PopupModel, Stepper, Swatch, Swatches, ToolbarModel,
    ToolbarSection,
};

use super::glyphs::{self, glyph, ink};
use super::run;
use crate::canvas;
use crate::theme;

/// The side of a popup control, and of a swatch button inside one.
const CONTROL: f32 = 24.0;
const SWATCH: f32 = 20.0;
const DOT: f32 = 12.0;
const GLYPH: f32 = 14.0;

/// A stable element id from a control's name.
fn element_id(id: &ControlId) -> SharedString {
    SharedString::from(id.as_str().to_owned())
}

/// What a control is called, with the key that does the same.
pub(super) fn hint(label: &str, chord: Option<Chord>) -> SharedString {
    match chord {
        Some(chord) => format!("{label}  {}", chord.text()).into(),
        None => label.to_owned().into(),
    }
}

/// What a control shows: its glyph, its word, or a dot of its colour.
fn face(face: &Face, on: bool) -> AnyElement {
    let tint =
        (face.color.as_ref()).map(|color| glyphs::resolved(color, Palette::Vivid, PaintRole::Ink));
    let mut row = h_flex().gap_1().items_center();
    if let Some(icon) = face.icon {
        row = row.child(glyph(icon, ink(theme::TEXT), tint, on, GLYPH));
    }
    if let Some(text) = &face.text {
        row = row.child(div().child(SharedString::from(text.to_string())));
    }
    if face.icon.is_none() && face.text.is_none() {
        row = row.child(dot(
            face.color.as_ref(),
            Palette::Vivid,
            PaintRole::Ink,
            16.0,
        ));
    }
    row.into_any_element()
}

/// A dot of `color`, or the no-colour glyph when there is none.
fn dot(
    color: Option<&specular_doc::Color>,
    palette: Palette,
    role: PaintRole,
    size: f32,
) -> AnyElement {
    match color {
        Some(color) => div()
            .size(px(size))
            .rounded_full()
            .bg(theme::of_scene(glyphs::resolved(color, palette, role)))
            .border_1()
            .border_color(theme::tinted(theme::DOT_EDGE))
            .into_any_element(),
        None => glyph(
            specular_interact::Icon::Ban,
            ink(theme::RING_GRAY),
            None,
            false,
            size,
        )
        .into_any_element(),
    }
}

fn button(model: &specular_interact::Button) -> AnyElement {
    let action = model.action.clone();
    Button::new(element_id(&model.id))
        .ghost()
        .xsmall()
        .h(px(CONTROL))
        .min_w(px(CONTROL))
        .tooltip(hint(&model.label, model.chord))
        .disabled(!model.enabled)
        .child(face(&model.face, false))
        .on_click(move |_, window, cx| run(&action, window, cx))
        .into_any_element()
}

fn toggle(model: &specular_interact::Toggle) -> AnyElement {
    let action = model.action.clone();
    Toggle::new(element_id(&model.id))
        .ghost()
        .xsmall()
        .h(px(CONTROL))
        .min_w(px(CONTROL))
        .checked(model.on)
        .tooltip(hint(&model.label, model.chord))
        .disabled(!model.enabled)
        .child(face(&model.face, model.on))
        .on_click(move |_, window, cx| run(&action, window, cx))
        .into_any_element()
}

/// How light a colour looks, 0 to 1.
fn luminance(color: specular_scene::Color) -> f32 {
    (0.2126 * f32::from(color.r) + 0.7152 * f32::from(color.g) + 0.0722 * f32::from(color.b))
        / 255.0
}

fn swatch(model: &Swatch, palette: Palette, role: PaintRole, enabled: bool) -> AnyElement {
    let action = model.action.clone();
    let label = SharedString::from(model.label.to_string());
    // A swatch rings itself in its own colour, unless it is too pale to
    // see (`swatchRingColor` in `colorSwatchStyle.ts`).
    let ring = (model.color.as_ref())
        .map(|color| glyphs::resolved(color, palette, role))
        .filter(|color| luminance(*color) <= 0.92)
        .map_or(theme::solid(theme::RING_GRAY), theme::of_scene);
    div()
        .id(element_id(&model.id))
        .size(px(SWATCH))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .when(model.selected, |this| this.border_2().border_color(ring))
        .when(!enabled, |this| this.opacity(0.3))
        .when(enabled, |this| {
            this.cursor_pointer()
                .on_click(move |_, window, cx| run(&action, window, cx))
        })
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .child(dot(model.color.as_ref(), palette, role, DOT))
        .into_any_element()
}

fn swatches(model: &Swatches) -> AnyElement {
    h_flex()
        .gap_1()
        .children(
            (model.options.iter())
                .map(|option| swatch(option, model.palette, model.role, model.enabled)),
        )
        .into_any_element()
}

fn stepper(model: &Stepper) -> AnyElement {
    let (decrement, increment) = (model.decrement.clone(), model.increment.clone());
    let step = |part: &str, icon: IconName, enabled: bool, action: Action| {
        Button::new(element_id(&model.id.child(part)))
            .ghost()
            .xsmall()
            .icon(icon)
            .disabled(!enabled)
            .on_click(move |_, window, cx| run(&action, window, cx))
    };
    h_flex()
        .id(element_id(&model.id))
        .gap_0p5()
        .items_center()
        .child(step("dec", IconName::Minus, model.can_decrement, decrement))
        .child(
            div()
                .min_w(px(28.0))
                .text_center()
                .child(SharedString::from(model.value.to_string())),
        )
        .child(step("inc", IconName::Plus, model.can_increment, increment))
        .into_any_element()
}

/// One row of a dropdown's list: the option's face, its small text at the
/// far end, and a fill when it is the current value.
fn option(model: &DropdownOption, wide: bool, dismiss: Dismiss) -> AnyElement {
    let action = model.action.clone();
    let trailing = (model.trailing.as_ref()).map(|text| SharedString::from(text.to_string()));
    let keys = model.chord.map(Chord::text);
    div()
        .id(element_id(&model.id))
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .h(px(if wide { CONTROL } else { 32.0 }))
        .when(wide, |this| this.w_full().px_2())
        .when(!wide, |this| this.w(px(32.0)).justify_center())
        .rounded(px(6.0))
        .cursor_pointer()
        .when(model.selected, |this| {
            this.bg(theme::solid(theme::CONTROL_ON))
        })
        .hover(|this| this.bg(theme::solid(theme::CONTROL_HOVER)))
        .child(face(&model.face, model.selected))
        .when_some(trailing, |this, text| {
            this.child(
                div()
                    .text_color(theme::tinted(theme::TEXT_MUTED))
                    .child(text),
            )
        })
        .when_some(keys, |this, keys| {
            this.child(
                div()
                    .px_1p5()
                    .py_0p5()
                    .rounded(px(4.0))
                    .bg(theme::solid(theme::CONTROL_ON))
                    .text_color(theme::solid(0x0057_534d))
                    .child(SharedString::from(keys)),
            )
        })
        .on_click(move |_, window, cx| {
            run(&action, window, cx);
            dismiss(window, cx);
        })
        .into_any_element()
}

/// Closes the list a chosen option was in.
type Dismiss = std::rc::Rc<dyn Fn(&mut gpui_kit::Window, &mut App)>;

fn section(model: &DropdownSection, dismiss: &Dismiss) -> AnyElement {
    match model {
        DropdownSection::Options { layout, options } => match layout {
            OptionLayout::List => v_flex()
                .gap_0p5()
                .min_w(px(140.0))
                .children(options.iter().map(|one| option(one, true, dismiss.clone())))
                .into_any_element(),
            OptionLayout::Row => h_flex()
                .gap_0p5()
                .children(
                    options
                        .iter()
                        .map(|one| option(one, false, dismiss.clone())),
                )
                .into_any_element(),
            OptionLayout::Grid { columns } => {
                let width = f32::from(*columns) * 32.0 + f32::from(columns.saturating_sub(1)) * 2.0;
                h_flex()
                    .flex_wrap()
                    .gap(px(2.0))
                    .w(px(width))
                    .children(
                        options
                            .iter()
                            .map(|one| option(one, false, dismiss.clone())),
                    )
                    .into_any_element()
            }
        },
        DropdownSection::Controls(controls) => h_flex()
            .gap_1()
            .items_center()
            .children(controls.iter().map(control))
            .into_any_element(),
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
        let popup: &PopupModel = models.popup.as_ref()?;
        among(&popup.controls, id)
    })
}

/// A dropdown: a trigger showing the current value, and its sections in a
/// Kit popover. It has no tooltip, which would sit over the open list.
/// `toolbar` is the zoom readout, as tall as a tool button.
pub(super) fn dropdown(model: &Dropdown, toolbar: bool) -> AnyElement {
    let id = model.id.clone();
    let trigger = Button::new(element_id(&model.id.child("trigger")))
        .ghost()
        .xsmall()
        .h(px(if toolbar { 28.0 } else { CONTROL }))
        .dropdown_caret(true)
        .child(face(&model.summary, false));
    Popover::new(element_id(&model.id))
        .trigger(trigger)
        .content(move |_, _, cx| {
            let popover = cx.entity();
            let dismiss: Dismiss = std::rc::Rc::new(move |window, cx| {
                popover.update(cx, |state, cx| state.dismiss(window, cx));
            });
            let sections = find_dropdown(&id).map(|dropdown| dropdown.content);
            let sections = sections.unwrap_or_default();
            let last = sections.len().saturating_sub(1);
            v_flex()
                .gap_1()
                .text_size(px(12.0))
                .text_color(cx.theme().popover_foreground)
                .children(sections.iter().enumerate().map(|(index, one)| {
                    v_flex()
                        .gap_1()
                        .child(section(one, &dismiss))
                        .when(index < last, |this| this.child(Separator::horizontal()))
                }))
        })
        .into_any_element()
}

/// A field's value as a line of text. Every field the models have sits in a
/// popup beside a canvas item, which the canvas's own pass draws and edits,
/// so nothing types into this one.
fn field(model: &Field) -> AnyElement {
    let text = match (&model.placeholder, model.value.is_empty()) {
        (Some(placeholder), true) => placeholder.to_string(),
        (Some(_), false) | (None, _) => model.value.clone(),
    };
    h_flex()
        .gap_1()
        .items_center()
        .text_size(px(12.0))
        .children(model.caption.as_ref().map(ToString::to_string))
        .child(text)
        .into_any_element()
}

/// A list shown in place: its sections one under another, as an open
/// dropdown has them. A choice leaves the list where it is.
fn choices(model: &Choices) -> AnyElement {
    let stay: Dismiss = std::rc::Rc::new(|_, _| {});
    let last = model.content.len().saturating_sub(1);
    v_flex()
        .gap_1()
        .text_size(px(12.0))
        .children(model.content.iter().enumerate().map(|(index, one)| {
            v_flex()
                .gap_1()
                .child(section(one, &stay))
                .when(index < last, |this| this.child(Separator::horizontal()))
        }))
        .into_any_element()
}

/// Any model control as a Kit component.
pub(super) fn control(model: &Control) -> AnyElement {
    match model {
        Control::Button(model) => button(model),
        Control::Toggle(model) => toggle(model),
        Control::Swatches(model) => swatches(model),
        Control::Dropdown(model) => dropdown(model, false),
        Control::Stepper(model) => stepper(model),
        Control::Field(model) => field(model),
        Control::Choices(model) => choices(model),
        Control::Separator => div()
            .mx_1()
            .w(px(1.0))
            .h(px(16.0))
            .bg(theme::tinted(theme::DIVIDER))
            .into_any_element(),
    }
}
