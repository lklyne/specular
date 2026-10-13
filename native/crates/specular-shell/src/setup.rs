//! The setup dialog (`src/renderer/onboarding`): installs the `specular`
//! command and the skill an agent drives the app with, when the user says
//! so. It opens by itself until setup is finished, and from Specular > Set
//! up Specular… after that.
//!
//! What is installed is on disk and not in the app's model, so the dialog
//! reads it through [`specular_app::agent_tools`] when it opens and after
//! each install.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::{
    App, FontWeight, Hsla, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div,
    px,
};
use specular_app::agent_tools::{self, Setup, Status};

use crate::assets::ShellIcon;
use crate::theme;

struct State {
    setup: Result<Setup, &'static str>,
    /// What the last install said went wrong.
    error: Option<String>,
}

impl State {
    fn read() -> Self {
        Self {
            setup: agent_tools::setup(),
            error: None,
        }
    }

    fn all_installed(&self) -> bool {
        (self.setup.as_ref())
            .is_ok_and(|setup| setup.command.installed() && setup.skill.installed())
    }
}

/// One of the two things the dialog installs.
struct Item {
    /// Its place in [`agent_tools::install`]'s arguments: the command, then
    /// the skill.
    index: usize,
    icon: ShellIcon,
    title: &'static str,
    detail: &'static str,
    /// What the pill and the button say when something else is in its
    /// place.
    differs: &'static str,
    replace: &'static str,
}

const COMMAND: Item = Item {
    index: 0,
    icon: ShellIcon::SquareTerminal,
    title: "Specular command",
    detail: "Adds the specular command, so agents can work with the app.",
    differs: "Another install",
    replace: "Replace",
};

const SKILL: Item = Item {
    index: 1,
    icon: ShellIcon::Sparkles,
    title: "Specular skill",
    detail: "Teaches Claude Code how to use Specular and edit the canvas.",
    differs: "Update available",
    replace: "Update",
};

fn muted() -> Hsla {
    theme::tinted(theme::text_muted())
}

/// A card's edge, lighter than the dialog's own.
fn rule() -> Hsla {
    theme::tinted(theme::divider()).opacity(0.6)
}

fn note(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .text_color(muted())
        .child(text.into())
}

/// The pill beside a card's title.
fn status_tag(item: &Item, status: &Status) -> Tag {
    match status {
        Status::Installed(_) => Tag::success().child("Installed"),
        Status::Missing => Tag::secondary().child("Not installed"),
        Status::Differs(_) => Tag::warning().child(item.differs),
    }
    .rounded_full()
}

/// What a card says under its description: where the installed one is, or
/// what is in its place.
fn status_note(status: &Status) -> Option<String> {
    match status {
        Status::Installed(place) => place.as_ref().map(|place| format!("Installed at {place}")),
        Status::Missing => None,
        Status::Differs(what) => Some(format!("{what}.")),
    }
}

fn card(
    item: &Item,
    status: &Status,
    state: &Rc<RefCell<State>>,
    cx: &App,
) -> impl IntoElement + use<> {
    let index = item.index;
    let install = Rc::clone(state);
    let recheck = Rc::clone(state);
    let action = match status {
        Status::Installed(_) => None,
        Status::Missing => Some("Install"),
        Status::Differs(_) => Some(item.replace),
    };
    v_flex()
        .w_full()
        .gap_3()
        .p_4()
        .border_1()
        .border_color(rule())
        .rounded(cx.theme().radius_lg)
        .child(
            h_flex()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(32.0))
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(rule())
                        .bg(cx.theme().muted)
                        .child(Icon::new(item.icon).small()),
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(item.title),
                )
                .child(status_tag(item, status)),
        )
        .child(
            v_flex()
                .gap_0p5()
                .child(note(item.detail))
                .children(status_note(status).map(note)),
        )
        .child(
            h_flex()
                .gap_2()
                .children(action.map(|label| {
                    Button::new(SharedString::from(format!("setup-install-{index}")))
                        .small()
                        .outline()
                        .icon(Icon::new(ShellIcon::Download))
                        .child(label)
                        .on_click(move |_, _, cx| {
                            let error = agent_tools::install(index == 0, index == 1).err();
                            let mut state = install.borrow_mut();
                            state.setup = agent_tools::setup();
                            state.error = error;
                            drop(state);
                            cx.refresh_windows();
                        })
                }))
                .child(
                    Button::new(SharedString::from(format!("setup-recheck-{index}")))
                        .small()
                        .ghost()
                        .icon(Icon::new(ShellIcon::RefreshCw))
                        .child("Re-check")
                        .on_click(move |_, _, cx| {
                            *recheck.borrow_mut() = State::read();
                            cx.refresh_windows();
                        }),
                ),
        )
}

fn content(state: &Rc<RefCell<State>>, cx: &App) -> impl IntoElement + use<> {
    let body = v_flex().w_full().gap_3().child(note(
        "Agents work with Specular through a command and a skill. Install them here, or \
         skip this and come back from the Specular menu.",
    ));
    let setup = state.borrow().setup.clone();
    let setup = match setup {
        Ok(setup) => setup,
        Err(why) => return body.child(note(why)),
    };
    let error = state.borrow().error.clone();
    body.children((!setup.claude).then(|| {
        note(
            "Claude Code does not seem to be set up (no ~/.claude folder). The skill can \
             still be installed, and is read once Claude Code is.",
        )
    }))
    .child(card(&COMMAND, &setup.command, state, cx))
    .child(card(&SKILL, &setup.skill, state, cx))
    .children(error.map(note))
}

fn footer(state: &Rc<RefCell<State>>) -> impl IntoElement + use<> {
    let installed = state.borrow().all_installed();
    let blocked = state.borrow().setup.is_err();
    h_flex()
        .w_full()
        .justify_between()
        .child(
            Button::new("setup-skip")
                .small()
                .ghost()
                .child(if installed || blocked {
                    "Close"
                } else {
                    "Skip for now"
                })
                .on_click(|_, window, cx| window.close_dialog(cx)),
        )
        // Done finishes setup, so the dialog does not open by itself again.
        .children((!blocked).then(|| {
            Button::new("setup-done")
                .small()
                .primary()
                .child("Done")
                .on_click(|_, window, cx| {
                    agent_tools::finish_setup();
                    window.close_dialog(cx);
                })
        }))
}

/// Opens the dialog over the window.
pub(crate) fn open(window: &mut Window, cx: &mut App) {
    let state = Rc::new(RefCell::new(State::read()));
    window.open_dialog(cx, move |dialog, _, cx| {
        dialog
            .title("Set up Specular")
            .w(px(520.0))
            .child(content(&state, cx))
            .footer(footer(&state))
    });
}

/// How often a launch that wants setup looks for an open space to show it
/// over.
const SPACE_POLL: std::time::Duration = std::time::Duration::from_millis(500);

/// Opens the dialog once a space is open, when setup has not been finished
/// and there is something to install. The first-run view asks for a space
/// first, and this waits behind it.
pub(crate) fn open_when_wanted(cx: &mut App) {
    cx.spawn(async move |cx| {
        let wanted = cx
            .background_executor()
            .spawn(async { agent_tools::wants_setup() })
            .await;
        if !wanted {
            return;
        }
        loop {
            let asking = crate::canvas::with(|canvas| {
                specular_interact::onboarding(canvas.runtime.app()).is_some()
            });
            if asking == Some(false) {
                break;
            }
            cx.background_executor().timer(SPACE_POLL).await;
        }
        cx.update(|cx| {
            crate::shell::with_view(cx, |_, window, cx| open(window, cx));
        });
    })
    .detach();
}
