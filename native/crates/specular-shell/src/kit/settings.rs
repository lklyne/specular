//! The settings dialog (`git show bc5253fa:src/renderer/settings`), from
//! [`SettingsModel`]: General, Repos, Mods, Shortcuts and About.
//!
//! The dialog's content is rebuilt on every frame it is drawn, so each
//! pane is the model as it is then, and a control sends its row's action.

use gpui_kit::component::button::Button;
use gpui_kit::component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::{
    App, FontWeight, IntoElement, ParentElement as _, SharedString, StyleRefinement, Styled as _,
    Window, div, px,
};
use specular_interact::mods::ModRow;
use specular_interact::{
    AboutRow, Action, Event, GeneralPane, SettingChoice, SettingToggle, SettingsModel, ShortcutRow,
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
        .gap(crate::theme::space(3.0))
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

/// A switch beside its setting, which sends `action` when flipped.
fn switch(label: SharedString, detail: SharedString, on: bool, action: Action) -> SettingItem {
    SettingItem::new(
        label,
        SettingField::switch(
            move |_| on,
            move |_, cx| {
                canvas::dispatch(Event::Action(action.clone()));
                cx.refresh_windows();
            },
        ),
    )
    .description(detail)
}

fn toggle(toggle: &SettingToggle) -> SettingItem {
    switch(
        toggle.label.into(),
        toggle.detail.into(),
        toggle.on,
        toggle.action.clone(),
    )
}

fn mod_switch(row: &ModRow) -> SettingItem {
    switch(
        row.title.clone().into(),
        row.detail.clone().into(),
        row.on,
        row.action.clone(),
    )
}

fn mods(rows: &[ModRow]) -> SettingPage {
    let group = SettingGroup::new().description(
        "A mod turned off does not run: its tools, commands and items go, and a layout \
         that places it shows the app's own panel there.",
    );
    let group = if rows.is_empty() {
        group.item(SettingItem::render(|_, _, _| {
            value("No mods are installed.".to_owned())
        }))
    } else {
        group.items(rows.iter().map(mod_switch))
    };
    SettingPage::new("Mods")
        .icon(Icon::new(ShellIcon::Blocks))
        .group(group)
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
        .group(
            SettingGroup::new()
                .title("Cursors")
                .description(
                    "Hiding a cursor only stops drawing it. Agents keep working and synced \
                     pages keep following.",
                )
                .items(pane.cursors.iter().map(toggle)),
        )
}

fn shortcut(row: &ShortcutRow) -> impl IntoElement + use<> {
    h_flex()
        .gap(crate::theme::space(3.0))
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

/// Wide enough, with [`SIDEBAR_WIDTH`], that the page is past the Kit's
/// 480 point stacking width: each switch and choice sits on its setting's
/// line, at the right.
const DIALOG_WIDTH: f32 = 760.0;
const SIDEBAR_WIDTH: f32 = 180.0;

fn model() -> Option<SettingsModel> {
    canvas::with(|canvas| settings(canvas.runtime.app()))
}

/// The title row's height. The dialog's close button sits 8 points in from
/// its corner, which centres it in a row this tall.
const TITLE_HEIGHT: f32 = 40.0;
/// The title's inset, matching the sidebar's own, so the title, the search
/// field and the pages' names share one left edge.
const TITLE_INSET: f32 = 12.0;

/// Opens the dialog over the window: the title, a rule the dialog's width
/// under it, then the sidebar and the page, edge to edge.
pub(crate) fn open(window: &mut Window, cx: &mut App) {
    let fields = AddFields::default();
    window.open_dialog(cx, move |dialog, _, cx| {
        let dialog = dialog.w(px(DIALOG_WIDTH)).p_0();
        let Some(model) = model() else {
            return dialog.child(title());
        };
        // The rule is under the title, so a page's name needs none of its
        // own.
        let page_header = StyleRefinement::default().border_b_0();
        let pages = [
            general(&model.general),
            settings_repos::page(&fields),
            mods(&model.mods),
            shortcuts(&model.shortcuts),
            about(&model.about),
        ]
        .map(|page| page.header_style(&page_header));
        dialog.child(
            v_flex()
                .child(title())
                .child(div().h(px(1.0)).w_full().bg(cx.theme().border))
                .child(
                    div().h(px(440.0)).child(
                        Settings::new("settings")
                            .sidebar_width(px(SIDEBAR_WIDTH))
                            .pages(pages),
                    ),
                ),
        )
    });
}

fn title() -> impl IntoElement {
    h_flex()
        .h(px(TITLE_HEIGHT))
        .px(px(TITLE_INSET))
        .items_center()
        .font_weight(FontWeight::SEMIBOLD)
        .child("Settings")
}
