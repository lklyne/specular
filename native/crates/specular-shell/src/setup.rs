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
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::{Disableable as _, Sizable as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::{
    App, FontWeight, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div, px,
};
use specular_app::agent_tools::{self, Setup, Status};

use crate::theme;

struct State {
    setup: Result<Setup, &'static str>,
    /// Whether each row is ticked, the command then the skill.
    picked: [bool; 2],
    /// What the last install said went wrong.
    error: Option<String>,
}

impl State {
    fn read() -> Self {
        let setup = agent_tools::setup();
        // A command that runs something else is someone's choice, so
        // replacing it is not ticked for them.
        let replaces =
            (setup.as_ref()).is_ok_and(|setup| matches!(setup.command, Status::Differs(_)));
        Self {
            setup,
            picked: [!replaces, true],
            error: None,
        }
    }

    /// Whether the command and the skill are each ticked and not there yet.
    fn to_install(&self) -> [bool; 2] {
        let Ok(setup) = &self.setup else {
            return [false, false];
        };
        [
            self.picked[0] && !setup.command.installed(),
            self.picked[1] && !setup.skill.installed(),
        ]
    }

    fn all_installed(&self) -> bool {
        (self.setup.as_ref())
            .is_ok_and(|setup| setup.command.installed() && setup.skill.installed())
    }
}

fn muted() -> gpui_kit::Hsla {
    theme::tinted(theme::text_muted())
}

fn note(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .text_color(muted())
        .child(text.into())
}

fn status_text(status: &Status) -> String {
    match status {
        Status::Installed(Some(place)) => format!("Installed at {place}"),
        Status::Installed(None) => "Installed".to_owned(),
        Status::Missing => "Not installed".to_owned(),
        Status::Differs(what) => format!("{what}. Installing replaces it."),
    }
}

fn row(
    index: usize,
    title: &'static str,
    detail: &'static str,
    status: &Status,
    state: &Rc<RefCell<State>>,
) -> impl IntoElement + use<> {
    let installed = status.installed();
    let picked = installed || state.borrow().picked[index];
    let on_change = Rc::clone(state);
    h_flex()
        .w_full()
        .gap_3()
        .items_start()
        .child(
            Checkbox::new(SharedString::from(format!("setup-pick-{index}")))
                .checked(picked)
                .disabled(installed)
                .on_click(move |on, _, cx| {
                    on_change.borrow_mut().picked[index] = *on;
                    cx.refresh_windows();
                }),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .child(note(detail))
                .child(note(status_text(status))),
        )
}

fn content(state: &Rc<RefCell<State>>) -> impl IntoElement + use<> {
    let body = v_flex().w_full().gap_4().child(note(
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
    .child(row(
        0,
        "Specular command",
        "Adds the specular command, so agents can work with the app.",
        &setup.command,
        state,
    ))
    .child(row(
        1,
        "Specular skill",
        "Teaches Claude Code how to use the command.",
        &setup.skill,
        state,
    ))
    .children(error.map(note))
}

fn footer(state: &Rc<RefCell<State>>) -> impl IntoElement + use<> {
    let installed = state.borrow().all_installed();
    let blocked = state.borrow().setup.is_err();
    // With nothing ticked that is still to install, the button finishes
    // setup, so the dialog does not open by itself again.
    let finished = state.borrow().to_install() == [false, false];
    let run = Rc::clone(state);
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
        .children((!blocked).then(|| {
            Button::new("setup-install")
                .small()
                .primary()
                .child(if finished { "Done" } else { "Install selected" })
                .on_click(move |_, window, cx| {
                    let [command, skill] = run.borrow().to_install();
                    if !command && !skill {
                        agent_tools::finish_setup();
                        window.close_dialog(cx);
                        return;
                    }
                    let error = agent_tools::install(command, skill).err();
                    let mut state = run.borrow_mut();
                    state.setup = agent_tools::setup();
                    state.error = error;
                    drop(state);
                    cx.refresh_windows();
                })
        }))
}

/// Opens the dialog over the window.
pub(crate) fn open(window: &mut Window, cx: &mut App) {
    let state = Rc::new(RefCell::new(State::read()));
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title("Set up Specular")
            .w(px(520.0))
            .child(content(&state))
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
