//! `specular mod clone`: a copy of a mod the app never writes over, made
//! with no app running, and the layout pointed at it with `--use`.

#![expect(clippy::expect_used, reason = "a test fails by panicking")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

/// An app home holding a shipped `tabs` mod and the shipped layout.
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
    let layout = json!({
        "tabs": { "mod": "tabs", "hidden": false, "props": { "hidden": [] } },
        "toolbar": { "mod": "toolbar", "hidden": false, "props": {} },
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
    let layout = read(&home.join("layouts/default.json"));
    assert_eq!(layout["tabs"]["mod"], "my-tabs");
    assert_eq!(
        layout["tabs"]["props"],
        json!({ "hidden": [] }),
        "the rest is kept"
    );
    assert_eq!(layout["toolbar"]["mod"], "toolbar");
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
