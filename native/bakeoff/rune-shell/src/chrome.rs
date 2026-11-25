//! The Rune view: a toolbar drawn straight from `toolbar(&App)`.
//!
//! Each control of the model is one Rune element, keyed by its `ControlId`,
//! with a role and the model's label as its accessible name. A press sends
//! the control's own `Action` through `update`. There is no layer between:
//! this file reads the model's fields and calls Rune's builders.
//!
//! Glyphs are words here. The real shell would draw `specular_scene`'s
//! icon svgs; that is drawing work, not a question the spike asks.

use rune::tok::type_;
use rune::{Color, Cx, El, HasPopup, col, el, row, text};
use specular_interact::{
    Action, Button, Control, ControlId, Dropdown, DropdownOption, DropdownSection, ThemeButton,
    Toggle, ToolButton, ToolbarModel, ToolbarSection, toolbar,
};

use crate::shell::Shell;

/// From the window's top edge to the ribbon.
const INSET: f32 = 8.0;
const BUTTON_HEIGHT: f32 = 28.0;

/// A colour from sRGB bytes. Rune's `Color` is linear with straight alpha.
fn srgb(r: u8, g: u8, b: u8, a: f32) -> Color {
    let linear = |byte: u8| {
        let encoded = f32::from(byte) / 255.0;
        if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    };
    Color {
        r: linear(r),
        g: linear(g),
        b: linear(b),
        a,
    }
}

pub(crate) fn surface() -> Color {
    srgb(252, 252, 251, 1.0)
}
pub(crate) fn edge() -> Color {
    srgb(0, 0, 0, 0.12)
}
pub(crate) fn ink() -> Color {
    srgb(60, 60, 58, 1.0)
}
pub(crate) fn ink_strong() -> Color {
    srgb(20, 20, 20, 1.0)
}
pub(crate) fn raised() -> Color {
    srgb(226, 232, 255, 1.0)
}
pub(crate) fn hover() -> Color {
    srgb(0, 0, 0, 0.06)
}

/// The last part of a control's name: `shape` of `tool.shape`.
fn short(id: &ControlId) -> &str {
    id.as_str().rsplit('.').next().unwrap_or(id.as_str())
}

/// One pressable control: keyed by its id, named by its label, showing
/// `word`, running `action`.
fn pressable(id: &ControlId, label: &str, word: &str, on: Option<bool>, action: Action) -> El {
    let lit = on == Some(true);
    let mut control = row()
        .key(id.as_str())
        .role("button")
        .label(label)
        .focusable(true)
        .h(BUTTON_HEIGHT)
        .px(8.0)
        .items_center()
        .radius(6.0)
        .fill(if lit { raised() } else { Color::TRANSPARENT })
        .hover_fill(if lit { raised() } else { hover() })
        .on_press(move |shell: &mut Shell| {
            shell.open_list = None;
            shell.act(action.clone());
        })
        .child(
            text(word)
                .font(type_::LABEL)
                .color(if lit { ink_strong() } else { ink() }),
        );
    if let Some(on) = on {
        control = control.checked(on);
    }
    control
}

fn tool(model: &ToolButton) -> El {
    pressable(
        &model.id,
        &model.label,
        short(&model.id),
        Some(model.active),
        model.action.clone(),
    )
}

fn theme(model: &ThemeButton) -> El {
    pressable(&model.id, &model.label, "theme", None, model.action.clone())
}

fn button(model: &Button) -> El {
    let word = model.face.text.as_deref().unwrap_or(short(&model.id));
    pressable(&model.id, &model.label, word, None, model.action.clone()).disabled(!model.enabled)
}

fn toggle(model: &Toggle) -> El {
    let word = model.face.text.as_deref().unwrap_or(short(&model.id));
    pressable(
        &model.id,
        &model.label,
        word,
        Some(model.on),
        model.action.clone(),
    )
    .disabled(!model.enabled)
}

fn option(model: &DropdownOption) -> El {
    let word = model.face.text.as_deref().unwrap_or(short(&model.id));
    row()
        .key(model.id.as_str())
        .role("menuitemradio")
        .label(model.label.as_ref())
        .checked(model.selected)
        .h(BUTTON_HEIGHT)
        .px(10.0)
        .items_center()
        .radius(4.0)
        .fill(if model.selected {
            raised()
        } else {
            Color::TRANSPARENT
        })
        .hover_fill(hover())
        .on_press({
            let action = model.action.clone();
            move |shell: &mut Shell| {
                shell.open_list = None;
                shell.act(action.clone());
            }
        })
        .child(text(word).font(type_::LABEL).color(ink()))
}

/// A dropdown: its trigger in the ribbon and, while it is open, its options
/// in a list under it. Which list is open is the renderer's own state.
fn dropdown(model: &Dropdown, open: bool) -> El {
    let id = model.id.clone();
    let word = model.summary.text.as_deref().unwrap_or(short(&model.id));
    let trigger = row()
        .key(model.id.as_str())
        .role("button")
        .label(model.label.as_ref())
        .has_popup(HasPopup::Menu)
        .focusable(true)
        .h(BUTTON_HEIGHT)
        .px(8.0)
        .items_center()
        .radius(6.0)
        .hover_fill(hover())
        .on_press(move |shell: &mut Shell| {
            shell.open_list = (shell.open_list.as_ref() != Some(&id)).then(|| id.clone());
        })
        .child(text(word).font(type_::LABEL).color(ink()));
    if !open {
        return trigger;
    }
    let options = model.content.iter().flat_map(|section| match section {
        DropdownSection::Options { options, .. } => options.iter().map(option).collect::<Vec<El>>(),
        DropdownSection::Controls(controls) => controls.iter().filter_map(control).collect(),
    });
    let list = col()
        .key(format!("{}.list", model.id))
        .role("menu")
        .label(model.label.as_ref())
        .absolute(0.0, BUTTON_HEIGHT + 6.0)
        .padding(4.0)
        .radius(8.0)
        .fill(surface())
        .border(1.0, edge())
        .children(options.collect::<Vec<El>>());
    el().child(trigger).child(list)
}

/// A control of a generic row. `None` for the kinds the toolbar never
/// holds, which the real shell's adapters would each need an arm for.
fn control(model: &Control) -> Option<El> {
    match model {
        Control::Button(model) => Some(button(model)),
        Control::Toggle(model) => Some(toggle(model)),
        Control::Dropdown(model) => Some(dropdown(model, false)),
        Control::Separator => Some(divider()),
        Control::Swatches(_) | Control::Stepper(_) | Control::Field(_) | Control::Choices(_) => {
            None
        }
    }
}

fn divider() -> El {
    el().w(1.0).h(16.0).fill(edge())
}

fn ribbon(model: &ToolbarModel, open_list: Option<&ControlId>) -> El {
    let mut items: Vec<El> = Vec::new();
    for section in &model.sections {
        if !items.is_empty() {
            items.push(divider());
        }
        match section {
            ToolbarSection::Tools(tools) => items.extend(tools.iter().map(tool)),
            ToolbarSection::Zoom(zoom) => items.push(dropdown(zoom, open_list == Some(&zoom.id))),
        }
    }
    items.push(theme(&model.theme));
    items.push(pressable(
        &model.share.copy.id,
        &model.share.copy.label,
        "share",
        None,
        model.share.copy.action.clone(),
    ));
    if let Some(chat) = &model.chat {
        items.push(pressable(
            &chat.id,
            &chat.label,
            "panel",
            Some(chat.open),
            chat.action.clone(),
        ));
    }
    if !model.view.is_empty() {
        items.push(divider());
        items.extend(model.view.iter().filter_map(control));
    }
    row()
        .key("toolbar")
        .role("toolbar")
        .label("Tools")
        .padding(4.0)
        .gap(2.0)
        .items_center()
        .radius(10.0)
        .fill(surface())
        .border(1.0, edge())
        .children(items)
}

/// The whole window: nothing but the ribbon, centred at the top. The rest
/// is left unpainted, and the canvas the compositor drew shows through.
///
/// With a scripted panel (`--chrome luau`) the whole window is the script's
/// instead: its root is this column and its ribbon is its own.
pub fn view(shell: &Shell, cx: &mut Cx<Shell>) -> El {
    if let Some(panel) = &shell.panel {
        return panel.view(shell, cx);
    }
    let model = toolbar(&shell.app);
    col()
        .w_full()
        .h_full()
        .items_center()
        .padding(INSET)
        .child(ribbon(&model, shell.open_list.as_ref()))
}
