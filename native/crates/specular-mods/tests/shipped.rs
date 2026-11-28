//! What the app writes into its mods folder on every start: the shipped
//! mods, replacing an older copy but never a person's clone, and a guide
//! and declarations that name the whole surface. And `specular mod check`'s
//! library half, over the shipped toolbar and over a mod that fails. A mod
//! the app stops shipping goes too, unless someone changed its folder.

use std::path::PathBuf;

use gpui::{TestAppContext, VisualTestContext};
use specular_interact::{STATE_KEYS, commands};
use specular_mods::{HOST_COMPONENTS, check, fresh_state, shipped};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("specular-mods-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn the_guide_and_the_declarations_name_every_command_key_and_component() {
    for (file, text) in [
        ("README.md", shipped::GUIDE),
        ("specular.d.ts", shipped::DECLARATIONS),
    ] {
        let names = (commands().iter().map(|spec| spec.name))
            .chain(STATE_KEYS)
            .chain(HOST_COMPONENTS);
        let missing: Vec<&str> = names
            .filter(|name| {
                !text.contains(&format!("`{name}`")) && !text.contains(&format!("\"{name}\""))
            })
            .collect();
        assert!(missing.is_empty(), "{file} does not name {missing:?}");
    }
}

#[test]
fn a_start_replaces_shipped_mods_but_not_a_clone() {
    let mods = scratch("install");
    shipped::install(&mods).expect("first start");
    let main = mods.join("toolbar/main.js");
    std::fs::write(&main, "// an edit").expect("edit");
    shipped::install(&mods).expect("second start");
    assert_eq!(
        std::fs::read_to_string(&main).expect("main.js"),
        shipped::named("toolbar").expect("shipped").files[1].text,
        "an edit to a shipped mod is replaced"
    );
    assert!(mods.join("README.md").is_file() && mods.join("specular.d.ts").is_file());

    let manifest = r#"{ "api": 1, "title": "Mine", "cloned_from": "toolbar" }"#;
    std::fs::write(mods.join("toolbar/mod.json"), manifest).expect("clone");
    std::fs::write(&main, "// mine").expect("edit");
    let kept = shipped::install(&mods).expect("third start");
    assert_eq!(kept, ["toolbar"]);
    assert_eq!(std::fs::read_to_string(&main).expect("main.js"), "// mine");
    let _ = std::fs::remove_dir_all(&mods);
}

#[test]
fn a_start_removes_a_mod_no_longer_shipped_unless_someone_changed_it() {
    let mods = scratch("retire");
    shipped::install(&mods).expect("first start");
    let copy = |name: &str| {
        let dir = mods.join(name);
        std::fs::create_dir_all(&dir).expect("folder");
        for entry in std::fs::read_dir(mods.join("star"))
            .expect("star")
            .flatten()
        {
            std::fs::copy(entry.path(), dir.join(entry.file_name())).expect("copy");
        }
        dir
    };
    let retired = copy("retired");
    std::fs::write(retired.join("jsconfig.json"), "{}").expect("written on load");
    std::fs::write(mods.join("retired.error"), "failed").expect("error file");
    let edited = copy("edited");
    std::fs::write(edited.join("main.js"), "// mine").expect("edit");
    let added = copy("added");
    std::fs::write(added.join("notes.md"), "mine").expect("add");
    let unmarked = copy("unmarked");
    std::fs::remove_file(unmarked.join(".shipped")).expect("unmark");

    shipped::install(&mods).expect("second start");
    assert!(!retired.exists() && !mods.join("retired.error").exists());
    assert!(edited.is_dir() && added.is_dir() && unmarked.is_dir());
    assert!(mods.join("star/main.js").is_file(), "a shipped mod stays");
    let _ = std::fs::remove_dir_all(&mods);
}

#[gpui::test]
fn a_check_prints_the_tree_or_fails_with_the_reason(cx: &mut TestAppContext) {
    let mods = scratch("check");
    shipped::install(&mods).expect("installed");
    let broken = mods.join("broken");
    std::fs::create_dir_all(&broken).expect("broken mod");
    std::fs::write(
        broken.join("mod.json"),
        r#"{ "api": 1, "title": "Broken" }"#,
    )
    .expect("manifest");
    std::fs::write(
        broken.join("main.js"),
        "import { View } from \"gpui-kit\";\nexport default class B extends View { render() { return nothing(); } }\n",
    )
    .expect("main.js");
    let fixture = fresh_state();
    let window = cx.add_window(|_, _| gpui::Empty);
    let mut vcx = VisualTestContext::from_window(*window, cx);
    let data = mods.join(".data");
    let report = vcx
        .update(|window, cx| check(&mods.join("toolbar"), &data, &fixture, window, cx))
        .expect("the shipped toolbar checks");
    assert!(
        report.tree.contains("Button \"tool.select\""),
        "{}",
        report.tree
    );
    // A thread block draws its snapshot from its thread's own key.
    let mut blocks = fixture.clone();
    let item = serde_json::json!([{ "id": "b1", "mod": "thread", "width": 380, "height": 520,
        "props": { "thread": "t1" } }]);
    let thread = serde_json::json!({ "id": "t1", "title": "Layouts", "messages": [
        { "id": "m1", "role": "user", "text": "Sketch three layouts" }] });
    blocks.insert("canvas/mod_items".to_owned(), item);
    blocks.insert("chat/thread/t1".to_owned(), thread);
    let report = vcx
        .update(|window, cx| check(&mods.join("thread"), &data, &blocks, window, cx))
        .expect("the shipped thread block checks");
    let drew = (report.runs.iter()).find(|run| run.starts_with("item.draw"));
    let drew = drew.expect("the block's snapshot is drawn");
    assert!(
        drew.contains("Layouts") && drew.contains("Sketch three layouts"),
        "{drew}"
    );
    let error = vcx
        .update(|window, cx| check(&broken, &data, &fixture, window, cx))
        .expect_err("a mod whose render throws fails its check");
    assert!(format!("{error:#}").contains("nothing"), "{error:#}");
    let _ = std::fs::remove_dir_all(&mods);
}
