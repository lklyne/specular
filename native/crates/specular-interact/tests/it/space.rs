//! How a tab ref names a canvas: an id or an exact name, never a guess.
//! Canvases in use are in `fixtures/scenarios/canvases.txt`.

use specular_interact::TabRefError;
use specular_testkit::{TestApp, document};

#[test]
fn a_tab_ref_is_an_id_or_an_exact_name_and_never_a_guess() {
    let app = TestApp::with_space([
        ("Home", document([])),
        ("tab_1", document([])),
        (" Notes ", document([])),
    ]);
    let space = app.app().space();
    let name = |tab_ref: &str| space.resolve(tab_ref).map(|canvas| canvas.name.as_str());
    // An id wins over a canvas that has it as its name.
    assert_eq!(name("tab_1"), Ok("Home"));
    assert_eq!(name(" Notes"), Ok(" Notes "));
    assert_eq!(name("  "), Err(TabRefError::Empty));
    let unknown = name("home").expect_err("names are exact");
    assert_eq!(
        unknown.to_string(),
        "unknown tab 'home' \u{2014} available: tab_1 (Home), tab_2 (tab_1), tab_3 ( Notes )"
    );
}
