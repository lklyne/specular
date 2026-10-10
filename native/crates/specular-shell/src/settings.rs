//! The settings dialog (`src/renderer/settings`), from
//! [`SettingsModel`]: General, Repos, Shortcuts and About.
//!
//! The dialog's content is rebuilt on every frame it is drawn, so each
//! pane is the model as it is then, and a control sends its row's action.

use gpui_kit::component::button::Button;
use gpui_kit::component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::{Icon, IconName, Sizable as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::{
    App, FontWeight, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div, px,
};
use specular_interact::{
    AboutRow, Event, GeneralPane, SettingChoice, SettingToggle, SettingsModel, ShortcutRow,
    settings,
};

use crate::assets::ShellIcon;
use crate::canvas;
use crate::settings_repos::{self, AddFields, send};
use crate::theme;

fn muted() -> gpui_kit::Hsla {
    theme::tinted(theme::text_muted())
}

fn value(text: String) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .text_color(muted())
        .child(SharedString::from(text))
}

/// The space row: its name over its path, and the two buttons.
fn space(pane: &GeneralPane) -> impl IntoElement + use<> {
    let change = pane.change.clone();
    let (name, path) = pane.space.as_ref().map_or_else(
        || ("No space is open".to_owned(), String::new()),
        |space| (space.name.clone(), space.path.clone()),
    );
    let reveal = pane.space.as_ref().map(|space| space.reveal.clone());
    h_flex()
        .w_full()
        .gap_3()
        .items_center()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(SharedString::from(name)),
                )
                .child(
                    div()
                        .font_family(specular_compositor::MONO_FAMILY)
                        .text_size(px(11.0))
                        .text_color(muted())
                        .child(SharedString::from(path)),
                ),
        )
        .children(reveal.map(|reveal| {
            Button::new("settings-space-reveal")
                .small()
                .child("Reveal in Finder")
                .on_click(move |_, window, _| send(&reveal, window))
        }))
        .child(
            Button::new("settings-space-change")
                .small()
                .child("Change\u{2026}")
                .on_click(move |_, window, _| send(&change, window)),
        )
}

fn toggle(toggle: &SettingToggle) -> SettingItem {
    let (on, action) = (toggle.on, toggle.action.clone());
    SettingItem::new(
        toggle.label,
        SettingField::switch(
            move |_| on,
            move |_, cx| {
                canvas::dispatch(Event::Action(action.clone()));
                cx.refresh_windows();
            },
        ),
    )
    .description(toggle.detail)
}

fn choice(choice: &SettingChoice) -> SettingItem {
    let options = (choice.options.iter())
        .map(|option| (option.name.into(), option.label.into()))
        .collect();
    let now: SharedString = (choice.options.iter())
        .find(|option| option.on)
        .map_or("", |option| option.name)
        .into();
    let actions: Vec<_> = (choice.options.iter())
        .map(|option| (option.name, option.action.clone()))
        .collect();
    SettingItem::new(
        choice.label,
        SettingField::dropdown(
            options,
            move |_| now.clone(),
            move |name: SharedString, cx| {
                if let Some((_, action)) = actions.iter().find(|(option, _)| name == *option) {
                    canvas::dispatch(Event::Action(action.clone()));
                    cx.refresh_windows();
                }
            },
        ),
    )
    .description(choice.detail)
}

fn general(pane: &GeneralPane) -> SettingPage {
    let row = pane.clone();
    SettingPage::new("General")
        .icon(IconName::Settings)
        .group(
            SettingGroup::new()
                .title("Space")
                .description(
                    "The folder holding your canvases, images and notes. Changing it opens \
                     another folder and leaves this one as it is.",
                )
                .item(SettingItem::render(move |_, _, _| space(&row))),
        )
        .group(
            SettingGroup::new()
                .title("At launch")
                .items(pane.toggles.iter().map(toggle)),
        )
        .group(
            SettingGroup::new()
                .title("Layout")
                .item(choice(&pane.tools)),
        )
}

fn shortcut(row: &ShortcutRow) -> impl IntoElement + use<> {
    h_flex()
        .gap_3()
        .items_center()
        .py(px(3.0))
        .text_size(px(12.0))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(SharedString::from(row.label.to_string())),
        )
        .child(div().w(px(110.0)).text_color(muted()).child(row.place))
        .child(
            div()
                .w(px(64.0))
                .font_family(specular_compositor::MONO_FAMILY)
                .child(SharedString::from(row.keys.clone())),
        )
}

fn shortcuts(rows: &[ShortcutRow]) -> SettingPage {
    let rows = rows.to_vec();
    SettingPage::new("Shortcuts")
        .icon(Icon::new(ShellIcon::Keyboard))
        .group(
            SettingGroup::new().item(SettingItem::render(move |_, _, _| {
                v_flex().w_full().children(rows.iter().map(shortcut))
            })),
        )
}

fn about(rows: &[AboutRow]) -> SettingPage {
    let items = rows.iter().map(|row| {
        let version = row.version.clone();
        SettingItem::new(
            SharedString::from(row.name.clone()),
            SettingField::element(move |_: &_, _: &mut Window, _: &mut App| value(version.clone())),
        )
    });
    SettingPage::new("About")
        .icon(IconName::Info)
        .group(SettingGroup::new().items(items))
}

fn model() -> Option<SettingsModel> {
    canvas::with(|canvas| settings(canvas.runtime.app()))
}

/// Opens the dialog over the window.
pub(crate) fn open(window: &mut Window, cx: &mut App) {
    let fields = AddFields::default();
    window.open_dialog(cx, move |dialog, _, _| {
        let dialog = dialog.title("Settings").w(px(720.0));
        let Some(model) = model() else {
            return dialog;
        };
        dialog.child(
            div().h(px(440.0)).child(
                Settings::new("settings")
                    .page(general(&model.general))
                    .page(settings_repos::page(&fields))
                    .page(shortcuts(&model.shortcuts))
                    .page(about(&model.about)),
            ),
        )
    });
}
