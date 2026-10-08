//! The theme: the button steps the choice, the choice and the system's
//! appearance decide what is drawn, and pages are told which scheme to
//! report.

use specular_doc::ColorScheme;
use specular_interact::{Action, Appearance, Effect, Event, Property, Theme};
use specular_testkit::TestApp;

/// What the step since the last drain asked for about the theme, in order.
fn asked(app: &mut TestApp) -> Vec<String> {
    (app.take_effects().iter())
        .filter_map(|effect| match effect {
            Effect::SaveTheme(theme) => Some(format!("save {}", theme.key())),
            Effect::SetPageColorScheme { page, scheme } => Some(format!("{page} {scheme:?}")),
            _ => None,
        })
        .collect()
}

#[test]
fn the_theme_button_steps_the_choice_and_pages_follow_what_is_drawn() {
    let mut app = TestApp::with_pages(2);
    app.with_panels();
    app.act(Action::Select(vec![])).select(&["p2"]);
    app.act(Action::SetProperty(Property::ColorScheme(Some(
        ColorScheme::Light,
    ))));
    app.select(&[]);

    // The system choice draws what the system is.
    assert_eq!(
        (app.app().theme(), app.app().appearance()),
        (Theme::System, Appearance::Light)
    );
    asked(&mut app);
    app.send(Event::SystemAppearance(Appearance::Dark));
    assert_eq!(app.app().appearance(), Appearance::Dark);
    assert_eq!(asked(&mut app), ["p1 None", "p2 Some(Light)"]);

    // Light, dark, system: each press saves the choice, and pages hear of
    // it only when what is drawn changes. A page that sets its own scheme
    // keeps it.
    app.click_control("theme");
    assert_eq!(
        (app.app().theme(), app.app().appearance()),
        (Theme::Light, Appearance::Light)
    );
    assert_eq!(asked(&mut app), ["save light", "p1 None", "p2 Some(Light)"]);
    app.click_control("theme");
    assert_eq!(app.app().appearance(), Appearance::Dark);
    assert_eq!(asked(&mut app), ["save dark", "p1 None", "p2 Some(Light)"]);
    app.click_control("theme");
    assert_eq!(
        (app.app().theme(), app.app().appearance()),
        (Theme::System, Appearance::Dark)
    );
    assert_eq!(asked(&mut app), ["save system"]);

    // A choice read from the preferences is not written back.
    app.send(Event::ThemeLoaded(Theme::Light));
    assert_eq!(app.app().appearance(), Appearance::Light);
    assert_eq!(asked(&mut app), ["p1 None", "p2 Some(Light)"]);
}
