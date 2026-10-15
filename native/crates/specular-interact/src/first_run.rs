//! The first run, and a space that is gone: what the app shows while no
//! space is open, and the choices it offers (ADR 0033).
//!
//! The app never picks a folder for the user. With none chosen it asks, and
//! with one chosen that is not there it asks again, naming the folder. Each
//! choice is an [`Action::Space`], and the folder work comes back as
//! effects.

use crate::{Action, App, Effect};

/// Why no space is open, as the shell found it at launch.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpaceAsk {
    /// The folder that was chosen before and is not there now. `None` on a
    /// first run.
    pub missing: Option<String>,
    /// The folder the Electron app has open, when it has one and it is
    /// there. Offered, never taken.
    pub electron: Option<String>,
}

/// Something done about which space is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceAction {
    /// Ask for a folder and open it as the space. `create` only words the
    /// dialog: a folder with no canvas gets the starter space either way,
    /// and one with canvases is opened as it is.
    Choose {
        /// Whether the user is making a space, not finding one.
        create: bool,
    },
    /// Open the folder at this path as the space, and remember it.
    Use(String),
    /// Show the open space's folder in the file manager.
    Reveal,
    /// Quit without opening anything.
    Quit,
}

/// One choice of the first-run view. The first two are its buttons, and
/// any after them are quieter links below.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceChoice {
    /// The control's name, for a scripted run.
    pub name: &'static str,
    /// The button's text.
    pub label: &'static str,
    /// What choosing it does, in a sentence: the button's tooltip.
    pub detail: String,
    /// Whether it is the choice the view leads with.
    pub primary: bool,
    /// What it runs.
    pub action: Action,
}

/// The view shown in place of the canvas while no space is open.
#[derive(Debug, Clone, PartialEq)]
pub struct OnboardingModel {
    /// The heading.
    pub title: &'static str,
    /// The line under the heading: what Specular is, or what happened to
    /// the space that is gone.
    pub body: Vec<String>,
    /// The choices, in order.
    pub choices: Vec<SpaceChoice>,
    /// A warning shown under the choices, when one of them needs it.
    pub note: Option<String>,
}

const WHAT_SPECULAR_IS: &str = "The browser whiteboard for designing and building with agents.";
const QUIT_THE_OTHER_APP: &str =
    "Quit the other Specular app first: two apps writing one space overwrite each other.";

fn choice(
    name: &'static str,
    label: &'static str,
    detail: &str,
    action: SpaceAction,
) -> SpaceChoice {
    SpaceChoice {
        name,
        label,
        detail: detail.to_owned(),
        primary: false,
        action: Action::Space(action),
    }
}

/// The first-run view for `app` as it is now, or `None` while a space is
/// open or the launch asked for none.
pub fn onboarding(app: &App) -> Option<OnboardingModel> {
    let ask = app.space_ask.as_ref()?;
    // One folder dialog both finds a space and makes one: it has its own
    // New Folder button, and a folder with no canvas gets the starter space.
    let choose = choice(
        "onboarding.choose",
        "Choose folder",
        "Pick a folder for your canvases, or make a new one. An empty folder starts with a \
         welcome canvas.",
        SpaceAction::Choose { create: false },
    );
    let electron = ask.electron.as_ref().map(|folder| {
        choice(
            "onboarding.electron",
            "Open existing space",
            &format!("Opens {folder}, the space you used in Specular before. {QUIT_THE_OTHER_APP}"),
            SpaceAction::Use(folder.clone()),
        )
    });
    // The warning is shown, not only the tooltip's, whenever that space is
    // offered: a user with the other app running must read it before opening.
    let note = (ask.missing.is_none() && electron.is_some()).then(|| QUIT_THE_OTHER_APP.to_owned());
    let (title, body, mut choices): (_, _, Vec<SpaceChoice>) = match &ask.missing {
        None => (
            "Welcome to Specular",
            vec![WHAT_SPECULAR_IS.to_owned()],
            electron.into_iter().chain([choose]).collect(),
        ),
        Some(folder) => (
            "Your space is not there",
            vec![format!(
                "{folder} could not be found. Its drive may not be connected, or the folder \
                 was moved or renamed. Nothing was opened in its place."
            )],
            vec![
                choice(
                    "onboarding.locate",
                    "Locate folder",
                    "Find where the space is now.",
                    SpaceAction::Choose { create: false },
                ),
                SpaceChoice {
                    label: "Choose another folder",
                    ..choose
                },
                choice(
                    "onboarding.quit",
                    "Quit",
                    "Leave everything as it is.",
                    SpaceAction::Quit,
                ),
            ],
        ),
    };
    if let Some(first) = choices.first_mut() {
        first.primary = true;
    }
    Some(OnboardingModel {
        title,
        body,
        choices,
        note,
    })
}

pub(crate) fn run(action: SpaceAction, effects: &mut Vec<Effect>) {
    effects.push(match action {
        SpaceAction::Choose { create } => Effect::ChooseSpace { create },
        SpaceAction::Use(folder) => Effect::OpenSpace(folder),
        SpaceAction::Reveal => Effect::RevealSpace,
        SpaceAction::Quit => Effect::Quit,
    });
}
