//! A model menu as a Kit popup menu: the context menu of a canvas row, and
//! the one a right press on the canvas opens.
//!
//! The model is a [`ControlsModel`] of choices, so an item's label, key,
//! enabled state and `Action` are the model's own.

use gpui_kit::component::h_flex;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::{InteractiveElement as _, ParentElement as _, SharedString, Styled as _, div, px};
use specular_interact::{Action, Chord, Control, ControlId, ControlsModel, DropdownSection};

use super::named::mark;
use crate::theme;

/// What picking an item does, given the item's name and its action.
pub(super) type Pick = fn(&ControlId, &Action, &mut gpui_kit::Window, &mut gpui_kit::App);

/// `menu` with the items of `model`, a line between its sections. A pick
/// runs `pick`.
pub(super) fn filled(mut menu: PopupMenu, model: &ControlsModel, pick: Pick) -> PopupMenu {
    let [Control::Choices(choices)] = model.controls.as_slice() else {
        return menu;
    };
    for (index, section) in choices.content.iter().enumerate() {
        let DropdownSection::Options { options, .. } = section else {
            continue;
        };
        if index > 0 {
            menu = menu.separator();
        }
        for option in options {
            let action = option.action.clone();
            let id = SharedString::from(option.id.as_str().to_owned());
            let name = option.id.clone();
            let option_id = option.id.clone();
            let label = SharedString::from(option.label.to_string());
            let keys = option.chord.map(Chord::text).map(SharedString::from);
            let item = PopupMenuItem::element(move |_, _| {
                h_flex()
                    .id(id.clone())
                    .w_full()
                    .gap_6()
                    .justify_between()
                    .text_size(px(12.0))
                    .child(label.clone())
                    .children(keys.clone().map(|keys| {
                        div()
                            .text_color(theme::tinted(theme::text_muted()))
                            .child(keys)
                    }))
                    .child(mark(&name))
            })
            .disabled(!option.enabled)
            .on_click(move |_, window, cx| pick(&option_id, &action, window, cx));
            menu = menu.item(item);
        }
    }
    menu
}
