//! The one place a model control becomes a Kit component.
//!
//! | Model       | Kit                                                      |
//! |-------------|----------------------------------------------------------|
//! | `Button`    | `Button`, ghost                                          |
//! | `Toggle`    | `Toggle`                                                 |
//! | `Swatches`  | a row of round buttons (the Kit has a picker, not a row) |
//! | `Dropdown`  | `Popover` over a `Button` trigger (`dropdown.rs`)        |
//! | `Choices`   | the same sections in place (`dropdown.rs`)               |
//! | `Stepper`   | two `Button`s around the value                           |
//! | `Field`     | `Input` (`field.rs`)                                     |
//! | `Separator` | a hairline                                               |
//!
//! Nothing here holds state. A control is drawn from the model each frame
//! and a click sends the model's own `Action` through `update`. Each
//! element's id is the model control's name.

use gpui_kit::component::button::{Button, ButtonVariants as _, Toggle, ToggleVariants as _};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Disableable as _, IconName, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use specular_doc::TextFont;
use specular_interact::{
    Action, Chord, Control, ControlId, Face, PaintRole, Palette, Stepper, Swatch, Swatches,
};

use super::dropdown::{choices, dropdown};
use super::field::field;
use super::glyphs::{self, glyph, ink};
use super::named::mark;
use super::run;
use crate::theme;

/// The side of a popup control, and of a swatch button inside one.
pub(super) const CONTROL: f32 = 24.0;
const SWATCH: f32 = 20.0;
const DOT: f32 = 12.0;
const GLYPH: f32 = 14.0;

/// A stable element id from a control's name.
pub(super) fn element_id(id: &ControlId) -> SharedString {
    SharedString::from(id.as_str().to_owned())
}

/// What a control is called, with the key that does the same.
pub(super) fn hint(label: &str, chord: Option<Chord>) -> SharedString {
    match chord {
        Some(chord) => format!("{label}  {}", chord.text()).into(),
        None => label.to_owned().into(),
    }
}

/// What a control shows: its glyph, its word, or a dot of its colour. As in
/// the built-in panels, a control that is on is drawn in the full text
/// colour and one at rest in the muted one.
pub(super) fn face(face: &Face, on: bool) -> AnyElement {
    let tint =
        (face.color.as_ref()).map(|color| glyphs::resolved(color, Palette::Vivid, PaintRole::Ink));
    let mut row = h_flex().gap_1().items_center();
    if let Some(icon) = face.icon {
        let current = ink(if on { theme::TEXT } else { theme::GLYPH_MUTED });
        row = row.child(glyph(icon, current, tint, on, GLYPH));
    }
    if let Some(text) = &face.text {
        let color = if on {
            theme::solid(theme::TEXT)
        } else {
            theme::tinted(theme::TEXT_MUTED)
        };
        row = row.child(
            div()
                .text_color(color)
                .when(face.font == Some(TextFont::Mono), |this| {
                    this.font_family("Menlo")
                })
                .child(SharedString::from(text.to_string())),
        );
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
        .child(mark(&model.id))
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
        .child(mark(&model.id))
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
        .child(mark(&model.id))
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
    // The mark is beside the button and not in it: a Kit button with a
    // child is no longer drawn as an icon alone.
    let step = |part: &str, icon: IconName, enabled: bool, action: Action| {
        let id = model.id.child(part);
        let button = Button::new(element_id(&id))
            .ghost()
            .xsmall()
            .icon(icon)
            .disabled(!enabled)
            .on_click(move |_, window, cx| run(&action, window, cx));
        div().child(button).child(mark(&id))
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

/// Any model control as a Kit component.
pub(super) fn control(model: &Control, window: &mut Window, cx: &mut App) -> AnyElement {
    match model {
        Control::Button(model) => button(model),
        Control::Toggle(model) => toggle(model),
        Control::Swatches(model) => swatches(model),
        Control::Dropdown(model) => dropdown(model, false),
        Control::Stepper(model) => stepper(model),
        Control::Field(model) => field(model, window, cx),
        Control::Choices(model) => choices(model, window, cx),
        Control::Separator => div()
            .mx_1()
            .w(px(1.0))
            .h(px(16.0))
            .bg(theme::tinted(theme::DIVIDER))
            .into_any_element(),
    }
}
