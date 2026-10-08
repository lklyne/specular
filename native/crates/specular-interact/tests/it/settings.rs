//! The settings dialog's model: the space folder, the launch defaults, the
//! repos, the shortcuts and the About rows.

use specular_doc::Document;
use specular_interact::{
    AboutRow, Action, AppSettings, BINDINGS, Effect, Event, SidebarAction, settings,
};
use specular_testkit::TestApp;

#[test]
fn the_general_pane_names_the_space_and_changes_or_reveals_it() {
    assert_eq!(settings(TestApp::empty().app()).general.space, None);

    let mut app = TestApp::with_space([("Welcome", Document::new())]);
    let general = settings(app.app()).general;
    let space = general.space.unwrap();
    assert_eq!(
        (space.name.as_str(), space.path.as_str()),
        ("space", "/space")
    );
    assert_eq!(app.act(space.reveal).take_effects(), [Effect::RevealSpace]);
    assert_eq!(
        app.act(general.change).take_effects(),
        [Effect::ChooseSpace { create: true }]
    );
}

#[test]
fn a_launch_default_is_saved_when_switched_and_shown_when_loaded() {
    let mut app = TestApp::empty();
    let toggles = settings(app.app()).general.toggles;
    let shown: Vec<_> = toggles
        .iter()
        .map(|toggle| (toggle.label, toggle.on))
        .collect();
    assert_eq!(
        shown,
        [
            ("Show the sidebar at launch", false),
            ("Show the right panel at launch", false),
        ]
    );

    // Switching one saves it and leaves the window as it is.
    let on = AppSettings {
        show_sidebar: true,
        show_chat: false,
    };
    assert_eq!(
        app.act(toggles[0].action.clone()).take_effects(),
        [Effect::SaveSettings(on)]
    );
    assert!(settings(app.app()).general.toggles[0].on);
    assert!(!app.session().sidebar.shown());
    // The same value again is no change and no write.
    assert_eq!(app.act(toggles[0].action.clone()).take_effects(), []);

    // The next launch reads it and shows the sidebar, with no save.
    let mut next = TestApp::empty();
    assert_eq!(next.send(Event::SettingsLoaded(on)).take_effects(), []);
    assert!(next.session().sidebar.shown());
    assert!(settings(next.app()).general.toggles[0].on);
    // Hiding it by hand does not change the default.
    next.act(Action::Sidebar(SidebarAction::Toggle));
    assert_eq!(next.app().settings(), on);
}

#[test]
fn the_shortcuts_are_the_binding_table_row_for_row_each_with_a_name() {
    let rows = settings(TestApp::empty().app()).shortcuts;
    assert_eq!(rows.len(), BINDINGS.len());
    for (row, binding) in rows.iter().zip(BINDINGS) {
        assert_eq!(row.keys, binding.chord.text());
        // A binding with no name would show as its action is written.
        assert!(
            !row.label.contains(['(', '{']),
            "no name for {:?}",
            binding.action
        );
    }
    let named = |keys: &str, place: &str| {
        (rows.iter())
            .find(|row| row.keys == keys && row.place == place)
            .map(|row| row.label.to_string())
    };
    assert_eq!(named("⇧⌘Z", "Canvas and text").as_deref(), Some("Redo"));
    assert_eq!(named("⇧R", "Canvas").as_deref(), Some("Diamond shape"));
    assert_eq!(named("⇧←", "Canvas").as_deref(), Some("Nudge left by 20"));
    assert_eq!(named("⌥⌘2", "Text").as_deref(), Some("Heading 2"));
    assert_eq!(named("⌘[", "Inside a page").as_deref(), Some("Back"));
    assert_eq!(named("⎋", "Everywhere").as_deref(), Some("Cancel"));
}

#[test]
fn the_repos_pane_switches_auto_fix_and_the_about_rows_are_the_shells() {
    let mut app = TestApp::empty();
    app.bind("https://example.com", "/scratch/site", false);
    app.take_effects();
    let origin = settings(app.app()).repos.repos[0].origins[0].clone();
    assert!(!origin.auto_fix);
    assert_eq!(
        app.act(origin.toggle_auto_fix).take_effects(),
        [Effect::SaveRepos]
    );
    assert!(settings(app.app()).repos.repos[0].origins[0].auto_fix);

    let rows = vec![AboutRow {
        name: "CEF".to_owned(),
        version: "154".to_owned(),
    }];
    app.send(Event::About(rows.clone()));
    assert_eq!(settings(app.app()).about, rows);
}
