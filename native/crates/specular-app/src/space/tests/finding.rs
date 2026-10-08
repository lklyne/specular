//! Finding a space and what it holds: the settings that name it, the
//! starter space, and the listing.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specular_doc::Document;
use specular_interact::CanvasId;

use super::{TempDir, canvas};
use crate::space::locate::{
    SpaceChoice, SpaceStart, electron_space, electron_user_data, scratch_folder, startup,
};
use crate::space::{listing, starter};

fn environment(
    pairs: &'static [(&'static str, &'static str)],
) -> impl Fn(&str) -> Option<OsString> {
    |name| {
        (pairs.iter())
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(value))
    }
}

#[test]
fn the_electron_apps_data_folder_is_where_electron_puts_it() {
    let mac = electron_user_data(environment(&[("HOME", "/Users/me")]), true);
    assert_eq!(
        mac,
        Some(PathBuf::from(
            "/Users/me/Library/Application Support/Specular"
        ))
    );
    let linux = electron_user_data(environment(&[("HOME", "/home/me")]), false);
    assert_eq!(linux, Some(PathBuf::from("/home/me/.config/Specular")));
    assert_eq!(electron_user_data(environment(&[]), true), None);
}

#[test]
fn the_space_is_the_electron_apps_space_path() {
    let dir = TempDir::new("electron-prefs");
    assert_eq!(electron_space(&dir.0), None);
    dir.write(
        "preferences.json",
        r#"{"devtoolsWidth":320,"spacePath":"/Users/me/Space"}"#,
    );
    assert_eq!(
        electron_space(&dir.0),
        Some(PathBuf::from("/Users/me/Space"))
    );
}

#[test]
fn with_no_space_path_it_is_the_electron_apps_old_fixed_folder() {
    let dir = TempDir::new("electron-legacy");
    dir.write("preferences.json", r#"{"devtoolsWidth":320}"#);
    dir.write("workspaces/default/Canvas 1-ab12.canvas", &canvas("a"));
    assert_eq!(
        electron_space(&dir.0),
        Some(dir.0.join("workspaces/default"))
    );
}

#[test]
fn a_launch_that_names_no_space_opens_the_scratch_space_and_never_the_users() {
    let dir = TempDir::new("startup-scratch");
    let (electron, remembered) = (dir.0.join("electron"), dir.0.join("remembered"));
    std::fs::create_dir_all(&electron).unwrap();
    std::fs::create_dir_all(&remembered).unwrap();
    let scratch = scratch_folder(Some(&dir.0));
    assert_eq!(scratch, dir.0.join("scratch-space"));
    assert_eq!(
        startup(
            &SpaceChoice::Scratch,
            Some(electron),
            Some(remembered),
            scratch.clone()
        ),
        Some(SpaceStart {
            folder: scratch,
            file: None,
            scratch: true,
        })
    );
}

#[test]
fn the_users_space_is_electrons_then_the_one_remembered_and_only_when_asked_for() {
    let dir = TempDir::new("startup");
    let (electron, remembered) = (dir.0.join("electron"), dir.0.join("remembered"));
    std::fs::create_dir_all(&electron).unwrap();
    std::fs::create_dir_all(&remembered).unwrap();
    let scratch = || dir.0.join("scratch-space");
    let folder = |folder: &Path| SpaceStart {
        folder: folder.to_owned(),
        file: None,
        scratch: false,
    };
    let both = || (Some(electron.clone()), Some(remembered.clone()));

    let (e, r) = both();
    assert_eq!(
        startup(&SpaceChoice::User, e, r, scratch()),
        Some(folder(&electron))
    );
    assert_eq!(
        startup(
            &SpaceChoice::User,
            None,
            Some(remembered.clone()),
            scratch()
        ),
        Some(folder(&remembered))
    );
    assert_eq!(startup(&SpaceChoice::User, None, None, scratch()), None);

    let (e, r) = both();
    let file = dir.0.join("other/Home.canvas");
    assert_eq!(
        startup(&SpaceChoice::Path(file), e, r, scratch()),
        Some(SpaceStart {
            folder: dir.0.join("other"),
            file: Some("Home.canvas".to_owned()),
            scratch: false,
        })
    );
    let (e, r) = both();
    assert_eq!(
        startup(&SpaceChoice::Path(dir.0.join("new space")), e, r, scratch()),
        Some(folder(&dir.0.join("new space")))
    );
}

#[test]
fn a_space_folder_from_settings_that_is_gone_is_not_opened_or_made() {
    let dir = TempDir::new("gone");
    let gone = dir.0.join("unmounted");
    assert_eq!(
        startup(
            &SpaceChoice::User,
            Some(gone.clone()),
            None,
            dir.0.join("scratch-space")
        ),
        None
    );
    assert!(!gone.exists());
}

#[test]
fn the_starter_space_is_copied_into_a_folder_with_no_canvas() {
    let (starter_dir, space) = (TempDir::new("starter"), TempDir::new("seeded"));
    starter_dir.write(
        "Welcome.canvas",
        r#"{"nodes":[{"id":"n","type":"file","file":"__SPECULAR_SPACE__/Welcome.md","x":0,"y":0,"width":1,"height":1}],"edges":[]}"#,
    );
    starter_dir.write("Welcome.md", "# Welcome\n");
    let target = space.0.join("new");
    assert!(starter::seed(&target, &starter_dir.0).unwrap());
    assert_eq!(
        std::fs::read_to_string(target.join("Welcome.md")).unwrap(),
        "# Welcome\n"
    );
    let seeded: Value =
        serde_json::from_str(&std::fs::read_to_string(target.join("Welcome.canvas")).unwrap())
            .unwrap();
    let note = target.join("Welcome.md");
    assert_eq!(seeded["nodes"][0]["file"], json!(note.to_string_lossy()));

    // A space with a canvas is never seeded, so a deleted starter stays
    // deleted.
    std::fs::remove_file(target.join("Welcome.md")).unwrap();
    assert!(!starter::seed(&target, &starter_dir.0).unwrap());
    assert!(!target.join("Welcome.md").exists());
}

#[test]
fn the_starter_space_that_ships_opens_as_a_canvas() {
    let space = TempDir::new("shipped-starter");
    let starter_dir = starter::starter_dir().expect("resources/starter-space is in the repository");
    assert!(starter::seed(&space.0, &starter_dir).unwrap());
    let document = Document::from_canvas_str(&space.read("Welcome.canvas")).unwrap();
    assert!(document.entities().count() > 0);
    assert!(!space.read("Welcome.canvas").contains("__SPECULAR_SPACE__"));
}

#[test]
fn the_listing_is_the_index_in_order_then_the_files_it_does_not_know() {
    let dir = TempDir::new("listing");
    dir.write(
        ".specular/workspace-meta.json",
        r#"{"activeTabId":"tab_bbbb2222","viewMode":"canvas","tabs":[
            {"id":"tab_aaaa1111","name":"Home","updatedAt":"2026-01-01T00:00:00.000Z","expanded":false},
            {"id":"tab_bbbb2222","name":"Old name","updatedAt":"2026-01-02T00:00:00.000Z"},
            {"id":"tab_cccc3333","name":"Deleted elsewhere","updatedAt":"2026-01-03T00:00:00.000Z"}]}"#,
    );
    dir.write("Home-aaaa.canvas", &canvas("a"));
    // From before file names carried part of the id.
    dir.write("Old name.canvas", &canvas("b"));
    // Not in the index: they arrived from outside the app.
    dir.write("Sketch-0f9e.canvas", &canvas("c"));
    dir.write("loose.canvas", &canvas("d"));
    dir.write("notes.md", "not a canvas");

    let listing = listing::list(&dir.0);
    let read: Vec<(&str, &str)> = (listing.canvases.iter())
        .map(|listed| (listed.name.as_str(), listed.file.as_str()))
        .collect();
    assert_eq!(
        read,
        [
            ("Home", "Home-aaaa.canvas"),
            ("Old name", "Old name.canvas"),
            ("Sketch", "Sketch-0f9e.canvas"),
            ("loose", "loose.canvas"),
        ]
    );
    assert_eq!(listing.active, Some(CanvasId::new("tab_bbbb2222")));
    // An adopted file's id leads back to the file, and is the same on the
    // next launch.
    let sketch = &listing.canvases[2];
    assert_eq!(
        specular_interact::canvas_file_name(&sketch.name, &sketch.id),
        sketch.file
    );
    assert_eq!(listing::list(&dir.0), listing);
}
