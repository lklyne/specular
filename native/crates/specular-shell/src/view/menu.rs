//! A model menu as a Kit popup menu: the context menu of a canvas row.
//!
//! The model is the one the built-in renderer draws, a [`PopupModel`] of
//! choices, so an item's label, key, enabled state and `Action` are the
//! model's own.

use gpui_kit::component::h_flex;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::{InteractiveElement as _, ParentElement as _, SharedString, Styled as _, div, px};
use specular_interact::{Chord, Control, DropdownSection, PopupModel};

use super::named::mark;
use super::run;
use crate::theme;

/// `menu` with the items of `model`, a line between its sections.
pub(super) fn filled(mut menu: PopupMenu, model: &PopupModel) -> PopupMenu {
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
            .on_click(move |_, window, cx| run(&action, window, cx));
            menu = menu.item(item);
        }
    }
    menu
}
