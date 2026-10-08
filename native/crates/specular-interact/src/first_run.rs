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

/// One button of the first-run view.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceChoice {
    /// The control's name, for a scripted run.
    pub name: &'static str,
    /// The button's text.
    pub label: &'static str,
    /// What choosing it does, in a sentence.
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
    /// What a space is, or what happened to the one that is gone.
    pub body: Vec<String>,
    /// The choices, in order.
    pub choices: Vec<SpaceChoice>,
}

const WHAT_A_SPACE_IS: &str = "A space is a folder on your computer. Every canvas is a .canvas \
    file in it, beside the images and notes you add. You choose where it lives, so you can open \
    it in Finder, back it up, or keep it in a repo.";
const NOTHING_IS_MOVED: &str = "Nothing is moved or copied from anywhere else.";

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
    let create = choice(
        "onboarding.create",
        "Create a new space\u{2026}",
        "Pick or make a folder. It starts with a welcome canvas.",
        SpaceAction::Choose { create: true },
    );
    let open = choice(
        "onboarding.open",
        "Open an existing folder\u{2026}",
        "Use a folder that already holds your canvases.",
        SpaceAction::Choose { create: false },
    );
    let electron = ask.electron.as_ref().map(|folder| {
        choice(
            "onboarding.electron",
            "Use the space from Specular",
            &format!(
                "Opens {folder}, the folder the Specular app uses. Quit Specular first: two apps \
                 writing one space overwrite each other."
            ),
            SpaceAction::Use(folder.clone()),
        )
    });
    let (title, body, mut choices) = match &ask.missing {
        None => (
            "Welcome to Specular",
            vec![WHAT_A_SPACE_IS.to_owned(), NOTHING_IS_MOVED.to_owned()],
            vec![create, open],
        ),
        Some(folder) => (
            "Your space is not there",
            vec![
                format!(
                    "{folder} could not be found. Its drive may not be connected, or the folder \
                     was moved or renamed."
                ),
                "Nothing was opened in its place.".to_owned(),
            ],
            vec![
                SpaceChoice {
                    name: "onboarding.locate",
                    label: "Locate the folder\u{2026}",
                    detail: "Find where the space is now.".to_owned(),
                    ..open
                },
                create,
            ],
        ),
    };
    choices.extend(electron);
    if ask.missing.is_some() {
        choices.push(choice(
            "onboarding.quit",
            "Quit",
            "Leave everything as it is.",
            SpaceAction::Quit,
        ));
    }
    if let Some(first) = choices.first_mut() {
        first.primary = true;
    }
    Some(OnboardingModel {
        title,
        body,
        choices,
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
