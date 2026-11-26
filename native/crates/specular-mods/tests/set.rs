//! The mods a layout names, as the window keeps them: loaded with their
//! props, a mod that fails shown as failed with `<name>.error` beside it,
//! and loaded once its files are fixed, with that file taken away.

use std::rc::Rc;

use gpui::{TestAppContext, VisualTestContext};
use serde_json::{Value, json};
use specular_interact::{Layout, RegionNode};
use specular_mods::{Hosting, ModHost, ModSet, Placed, RunError, shipped};

struct Quiet;

impl ModHost for Quiet {
    fn run(&self, _: &str, _: &Value) -> Result<Value, RunError> {
        Ok(Value::Null)
    }
    fn commands(&self) -> Value {
        json!([])
    }
    fn notify(&self, _: &str, _: &str) {}
}

const BROKEN: &str =
    "import { View } from \"gpui-kit\";\nexport default class B extends View { render( }\n";
const FIXED: &str = "import { View, div } from \"gpui-kit\";\nexport default class B extends View { render() { return div().child(\"dock\"); } }\n";

#[gpui::test]
fn a_mod_that_fails_says_why_until_its_files_are_fixed(cx: &mut TestAppContext) {
    let mods = std::env::temp_dir().join(format!("specular-mods-set-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&mods);
    shipped::install(&mods).expect("installed");
    let dock = mods.join("dock");
    std::fs::create_dir_all(&dock).expect("dock mod");
    std::fs::write(dock.join("mod.json"), r#"{ "api": 1, "title": "Dock" }"#).expect("manifest");
    std::fs::write(dock.join("main.js"), BROKEN).expect("main.js");

    let mut layout = Layout::shipped();
    layout.dock = RegionNode {
        fill: Some("dock".to_owned()),
        ..RegionNode::default()
    };
    let mut set = ModSet::new(mods.clone(), Hosting::new(Rc::new(Quiet)));
    let window = cx.add_window(|_, _| gpui::Empty);
    let mut vcx = VisualTestContext::from_window(*window, cx);
    vcx.update(|window, cx| set.sync(&layout, window, cx));
    vcx.update(|_, cx| {
        assert!(matches!(set.placed("toolbar", cx), Some(Placed::View(_))));
        assert!(matches!(set.placed("dock", cx), Some(Placed::Failed(_))));
    });
    assert!(
        shipped::error_file(&dock).is_file(),
        "dock.error is written"
    );
    assert_eq!(
        set.hosting().revisions.borrow().get("mods/toolbar/props"),
        Some(&json!({ "hidden": [] })),
        "the layout's props are published"
    );
    assert_eq!(set.palette().len(), 1, "the toolbar's palette entry");

    // A save that fixes it loads it.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(dock.join("main.js"), FIXED).expect("fix");
    let changed = vcx.update(|window, cx| set.sync(&layout, window, cx));
    assert!(changed, "the fixed mod loads");
    vcx.update(|_, cx| assert!(matches!(set.placed("dock", cx), Some(Placed::View(_)))));
    assert!(
        !shipped::error_file(&dock).exists(),
        "dock.error is taken away"
    );
    let _ = std::fs::remove_dir_all(&mods);
}
