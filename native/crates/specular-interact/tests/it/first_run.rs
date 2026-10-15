//! The first run and a missing space: what is offered while no space is
//! open, and what each choice asks of the shell.

use specular_doc::Document;
use specular_interact::{
    Action, Effect, Event, OnboardingModel, SpaceAction, SpaceAsk, onboarding,
};
use specular_testkit::{TestApp, space};

const ELECTRON: &str = "/Users/ada/Specular";

fn asked(ask: SpaceAsk) -> TestApp {
    let mut app = TestApp::empty();
    app.send(Event::SpaceNeeded(ask));
    app.take_effects();
    app
}

fn labels(model: &OnboardingModel) -> Vec<(&str, bool)> {
    (model.choices.iter())
        .map(|choice| (choice.label, choice.primary))
        .collect()
}

#[test]
fn a_first_run_welcomes_and_offers_a_folder_or_the_existing_space_and_never_picks() {
    assert_eq!(onboarding(TestApp::empty().app()), None);

    let app = asked(SpaceAsk::default());
    let model = onboarding(app.app()).unwrap();
    assert_eq!(model.title, "Welcome to Specular");
    assert_eq!(
        model.body,
        ["The browser whiteboard for designing and building with agents."]
    );
    assert_eq!(labels(&model), [("Choose folder", true)]);
    assert_eq!(model.note, None);

    // The Electron app's folder leads, named, and is not taken.
    let mut app = asked(SpaceAsk {
        missing: None,
        electron: Some(ELECTRON.to_owned()),
    });
    let model = onboarding(app.app()).unwrap();
    assert_eq!(
        labels(&model),
        [("Open existing space", true), ("Choose folder", false)]
    );
    assert!(model.choices[0].detail.contains(ELECTRON));
    // Its warning is on the view, not only in the button's tooltip.
    assert!(
        model
            .note
            .as_deref()
            .is_some_and(|note| note.starts_with("Quit the other Specular app first"))
    );
    assert_eq!(app.app().space().folder(), None);

    let effects: Vec<_> = (model.choices.iter())
        .flat_map(|choice| app.act(choice.action.clone()).take_effects())
        .collect();
    assert_eq!(
        effects,
        [
            Effect::OpenSpace(ELECTRON.to_owned()),
            Effect::ChooseSpace { create: false },
        ]
    );
    // Asking for a folder opens nothing: the view stays until one opens.
    assert!(onboarding(app.app()).is_some());
    app.send(Event::SpaceOpened(Box::new(space([(
        "Welcome",
        Document::new(),
    )]))));
    assert_eq!(onboarding(app.app()), None);
}

#[test]
fn a_space_that_is_gone_is_named_and_can_be_located_replaced_or_left() {
    let mut app = asked(SpaceAsk {
        missing: Some("/Volumes/Work/Design".to_owned()),
        electron: None,
    });
    let model = onboarding(app.app()).unwrap();
    assert_eq!(model.title, "Your space is not there");
    assert!(model.body[0].starts_with("/Volumes/Work/Design could not be found."));
    assert_eq!(
        labels(&model),
        [
            ("Locate folder", true),
            ("Choose another folder", false),
            ("Quit", false),
        ]
    );
    assert_eq!(
        app.act(model.choices[0].action.clone()).take_effects(),
        [Effect::ChooseSpace { create: false }]
    );
    assert_eq!(
        app.act(Action::Space(SpaceAction::Quit)).take_effects(),
        [Effect::Quit]
    );
}
