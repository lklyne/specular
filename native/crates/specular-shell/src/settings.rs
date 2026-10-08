//! The settings dialog. A stub: it shows where things are and changes
//! nothing yet. The Electron app's panes (General, Skills, Models, Repos)
//! each need state this app does not have.

use gpui_kit::component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::{IconName, WindowExt as _};
use gpui_kit::{App, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div, px};

use crate::canvas;
use crate::theme;

/// What the dialog reads from the running app.
#[derive(Debug, Clone, Default)]
struct Facts {
    space: String,
    backend: String,
}

fn facts() -> Facts {
    canvas::with(|canvas| {
        let space = canvas.runtime.space_folder();
        Facts {
            space: space.map_or_else(
                || "No space is open".to_owned(),
                |folder| folder.display().to_string(),
            ),
            backend: canvas.runtime.source_name().to_owned(),
        }
    })
    .unwrap_or_default()
}

fn value(text: String) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .text_color(theme::tinted(theme::TEXT_MUTED))
        .child(SharedString::from(text))
}

/// Opens the dialog over the window.
pub(crate) fn open(window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, |dialog, _, _| {
        let facts = facts();
        let general = SettingPage::new("General")
            .icon(IconName::Settings)
            .group(
                SettingGroup::new()
                    .title("Space")
                    .item(SettingItem::new(
                        "Folder",
                        SettingField::element({
                            let text = facts.space;
                            move |_: &_, _: &mut Window, _: &mut App| value(text.clone())
                        }),
                    ))
                    .item(SettingItem::new(
                        "Pages",
                        SettingField::element({
                            let text = facts.backend;
                            move |_: &_, _: &mut Window, _: &mut App| value(text.clone())
                        }),
                    )),
            )
            .group(SettingGroup::new().title("About").item(SettingItem::new(
                "Version",
                SettingField::element(|_: &_, _: &mut Window, _: &mut App| {
                    value(env!("CARGO_PKG_VERSION").to_owned())
                }),
            )));
        dialog.title("Settings").w(px(640.0)).child(
            div()
                .h(px(360.0))
                .child(Settings::new("settings").page(general)),
        )
    });
}
