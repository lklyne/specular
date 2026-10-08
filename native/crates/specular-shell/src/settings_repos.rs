//! The settings dialog's Repos pane (`ReposPane.tsx`): the folders the chat
//! panel may write to, and the origins bound to each.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage};
use gpui_kit::component::{Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, Entity, FontWeight, IntoElement, ParentElement as _, SharedString,
    Styled as _, Subscription, Window, div, px,
};
use specular_interact::{
    Action, BoundOriginRow, Event, RepoAction, RepoRow, ReposPane, repos_pane,
};

use crate::assets::ShellIcon;
use crate::canvas;
use crate::theme;

const INTRO: &str = "Bind a site to a local folder from the page popup (folder icon) or by adding a URL below. The chat panel writes to that repo when the current turn is about that origin.";

/// The "add a url" fields, one per repo, made when the repo first shows.
/// The dialog's content is rebuilt each frame, so the fields live here.
#[derive(Clone, Default)]
pub(crate) struct AddFields(Rc<RefCell<HashMap<String, AddField>>>);

/// A field and the subscription that listens for its Enter.
type AddField = (Entity<InputState>, Subscription);

impl AddFields {
    fn field(&self, repo: &str, window: &mut Window, cx: &mut App) -> Entity<InputState> {
        if let Some((input, _)) = self.0.borrow().get(repo) {
            return input.clone();
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("https://example.com"));
        let id = repo.to_owned();
        let subscription = window.subscribe(&input, cx, move |input, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                let origin = input.read(cx).value().trim().to_owned();
                if !origin.is_empty() {
                    input.update(cx, |state, cx| state.set_value("", window, cx));
                    send(
                        &Action::Repo(RepoAction::BindTo {
                            repo: id.clone(),
                            origin,
                        }),
                        window,
                    );
                }
            }
        });
        self.0
            .borrow_mut()
            .insert(repo.to_owned(), (input.clone(), subscription));
        input
    }
}

/// Runs `action` and has the dialog read the model again.
fn send(action: &Action, window: &mut Window) {
    canvas::dispatch(Event::Action(action.clone()));
    window.refresh();
}

fn muted() -> gpui_kit::Hsla {
    theme::tinted(theme::TEXT_MUTED)
}

fn origin_row(row: &BoundOriginRow, index: usize) -> impl IntoElement + use<> {
    let remove = row.remove.clone();
    h_flex()
        .gap_2()
        .items_center()
        .text_size(px(12.0))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(SharedString::from(row.origin.clone())),
        )
        .children(row.auto_fix.then(|| {
            div()
                .rounded_full()
                .bg(theme::tinted(theme::CHIP_HOVER))
                .px(px(6.0))
                .text_size(px(10.0))
                .text_color(muted())
                .child("auto")
        }))
        .child(
            Button::new(SharedString::from(format!("origin-remove-{index}")))
                .ghost()
                .xsmall()
                .icon(IconName::Close)
                .tooltip("Remove")
                .on_click(move |_, window, _| send(&remove, window)),
        )
}

fn repo_card(
    repo: &RepoRow,
    index: usize,
    fields: &AddFields,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let disconnect = repo.disconnect.clone();
    let input = fields.field(&repo.id, window, cx);
    v_flex()
        .gap_2()
        .rounded(px(8.0))
        .border_1()
        .border_color(theme::solid(theme::ZINC_300))
        .p_3()
        .child(
            h_flex()
                .gap_2()
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
                                .child(SharedString::from(repo.label.clone())),
                        )
                        .child(
                            div()
                                .font_family("Menlo")
                                .text_size(px(11.0))
                                .text_color(muted())
                                .truncate()
                                .child(SharedString::from(repo.path.clone())),
                        ),
                )
                .child(
                    Button::new(SharedString::from(format!("repo-disconnect-{index}")))
                        .ghost()
                        .xsmall()
                        .child("Disconnect")
                        .on_click(move |_, window, _| send(&disconnect, window)),
                ),
        )
        .children(
            repo.origins
                .iter()
                .enumerate()
                .map(|(at, row)| origin_row(row, index * 1000 + at)),
        )
        .child(Input::new(&input).xsmall())
}

fn pane_body(
    pane: &ReposPane,
    fields: &AddFields,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let connect = pane.connect.clone();
    v_flex()
        .gap_3()
        .child(div().text_size(px(12.0)).text_color(muted()).child(INTRO))
        .child(
            h_flex().child(
                Button::new("repos-connect")
                    .icon(Icon::new(ShellIcon::FolderCode))
                    .child("Connect repo…")
                    .on_click(move |_, window, _| send(&connect, window)),
            ),
        )
        .children(
            pane.repos
                .iter()
                .enumerate()
                .map(|(index, repo)| repo_card(repo, index, fields, window, cx))
                .collect::<Vec<_>>(),
        )
}

/// The Repos page, read from the model each time it is drawn.
pub(crate) fn page(fields: &AddFields) -> SettingPage {
    let fields = fields.clone();
    SettingPage::new("Repos")
        .icon(Icon::new(ShellIcon::FolderCode))
        .group(
            SettingGroup::new().item(SettingItem::render(move |_, window, cx| {
                let pane = canvas::with(|canvas| repos_pane(canvas.runtime.app()));
                div().children(pane.map(|pane| pane_body(&pane, &fields, window, cx)))
            })),
        )
}
