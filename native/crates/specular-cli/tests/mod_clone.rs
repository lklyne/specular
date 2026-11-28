//! `specular mod clone`: a copy of a mod the app never writes over, made
//! with no app running, and the layout pointed at it with `--use`.

#![expect(clippy::expect_used, reason = "a test fails by panicking")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

/// An app home holding shipped `tabs`, `toolbar` and `toast` mods and the
/// shipped layout.
fn home(name: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("specular-clone-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let tabs = home.join("mods/tabs");
    std::fs::create_dir_all(&tabs).expect("mods folder");
    std::fs::create_dir_all(home.join("layouts")).expect("layouts folder");
    let manifest = json!({ "api": 1, "title": "Tab row", "place": "tabs" });
    std::fs::write(tabs.join("mod.json"), manifest.to_string()).expect("mod.json");
    std::fs::write(tabs.join("main.js"), "// the tab row\n").expect("main.js");
    std::fs::write(tabs.join("gpui-kit.d.ts"), "// written on load\n").expect("d.ts");
    for name in ["toolbar", "toast"] {
        let folder = home.join("mods").join(name);
        std::fs::create_dir_all(&folder).expect("a mod folder");
        let manifest = json!({ "api": 1, "title": name });
        std::fs::write(folder.join("mod.json"), manifest.to_string()).expect("mod.json");
    }
    let layout = json!({
        "id": "root",
        "type": "column",
        "toast": { "mod": "toast" },
        "children": [
            { "id": "tabs", "type": "block",
              "content": { "mod": "tabs", "props": { "hidden": [] } } },
            { "id": "body", "type": "row", "children": [
                { "id": "toolbar", "type": "block",
                  "content": { "mod": "toolbar", "props": { "hidden": [] } } },
                { "id": "canvas", "type": "block", "content": "canvas" },
            ] },
        ],
    });
    std::fs::write(home.join("layouts/default.json"), layout.to_string()).expect("layout");
    home
}

fn clone(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_specular-cli"))
        .args(["mod", "clone"])
        .args(args)
        .env("SPECULAR_NATIVE_CONFIG_DIR", home)
        .output()
        .expect("the CLI runs")
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("a file")).expect("JSON")
}

#[test]
fn a_clone_is_marked_and_with_use_the_layout_names_it() {
    let home = home("use");
    let out = clone(&home, &["tabs", "--use"]);
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let copy = home.join("mods/my-tabs");
    assert!(
        stdout.contains(&copy.display().to_string()),
        "it says where: {stdout}"
    );
    assert_eq!(read(&copy.join("mod.json"))["cloned_from"], "tabs");
    assert_eq!(read(&copy.join("mod.json"))["title"], "Tab row");
    assert_eq!(
        std::fs::read_to_string(copy.join("main.js")).expect("main.js"),
        "// the tab row\n"
    );
    assert!(
        !copy.join("gpui-kit.d.ts").exists(),
        "what a load writes is left out"
    );
    let layout = read(&home.join("layouts/mine.json"));
    let tabs = &layout["children"][0]["content"];
    assert_eq!(tabs["mod"], "my-tabs", "the person's own layout");
    assert_eq!(tabs["props"], json!({ "hidden": [] }), "the rest is kept");
    let toolbar = &layout["children"][1]["children"][0]["content"];
    assert_eq!(toolbar["mod"], "toolbar");
    assert!(layout.get("my-tabs").is_none() && layout.get("tabs").is_none());
    assert!(
        stdout.contains("layout.switch"),
        "it says how to show it: {stdout}"
    );
    // A block in the tree, and an overlay at the top, are each found.
    for (from, at) in [
        (
            "toolbar",
            &["children", "1", "children", "0", "content"][..],
        ),
        ("toast", &["toast"][..]),
    ] {
        let out = clone(&home, &[from, "--use"]);
        assert!(out.status.success(), "{out:?}");
        let layout = read(&home.join("layouts/mine.json"));
        let node = at
            .iter()
            .fold(&layout, |node, key| match key.parse::<usize>() {
                Ok(index) => &node[index],
                Err(_) => &node[*key],
            });
        assert_eq!(node["mod"], format!("my-{from}"), "{layout}");
    }
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_clone_of_a_mod_the_layout_does_not_name_goes_in_its_place() {
    let home = home("place");
    let dock = home.join("mods/dock");
    std::fs::create_dir_all(&dock).expect("dock");
    let manifest = json!({ "api": 1, "title": "Dock", "place": "canvas" });
    std::fs::write(dock.join("mod.json"), manifest.to_string()).expect("mod.json");
    let out = clone(&home, &["dock", "--use"]);
    assert!(out.status.success(), "{out:?}");
    let layout = read(&home.join("layouts/mine.json"));
    let canvas = &layout["children"][1]["children"][1];
    assert_eq!(canvas["content"], json!({ "mod": "my-dock" }), "{layout}");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_clone_without_use_leaves_the_layout_and_refuses_what_it_cannot_do() {
    let home = home("plain");
    let before = std::fs::read_to_string(home.join("layouts/default.json")).expect("layout");
    let out = clone(&home, &["tabs", "row"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(read(&home.join("mods/row/mod.json"))["cloned_from"], "tabs");
    assert_eq!(
        std::fs::read_to_string(home.join("layouts/default.json")).expect("layout"),
        before
    );
    for (args, why) in [
        (&["tabs", "row"][..], "is there already"),
        (&["nope"], "no mod `nope`"),
        (&["tabs", "../out"], "is not a mod name"),
        (&[], "usage: specular mod clone"),
    ] {
        let out = clone(&home, args);
        assert!(!out.status.success(), "{args:?} is refused");
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(said.contains(why), "{args:?}: {said}");
    }
    let _ = std::fs::remove_dir_all(&home);
}
