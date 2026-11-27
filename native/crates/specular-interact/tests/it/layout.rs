//! The layout file (ADR 0055): what it reads, what it writes, and a bad save
//! that leaves the last good layout in force.

use serde_json::json;
use specular_interact::{Layout, LayoutFile, RegionNode, Toggles};

#[test]
fn a_layout_reads_back_as_written_with_its_props_untouched() {
    let shipped = Layout::shipped();
    assert_eq!(Layout::parse(&shipped.to_text()), Ok(shipped.clone()));
    let text = r#"{
        "toolbar": { "mod": "my-tools", "props": { "hidden": ["tool.inspect"], "size": [1, "x"] } },
        "sidebar": { "hidden": true }
    }"#;
    let layout = Layout::parse(text).expect("a layout");
    assert_eq!(layout.toolbar.fill.as_deref(), Some("my-tools"));
    assert_eq!(layout.toolbar.props["size"], json!([1, "x"]));
    assert!(layout.hides("sidebar") && layout.hides("tool.inspect"));
    assert!(!layout.hides("tabs") && !layout.hides("tool.select"));
    assert_eq!(
        layout.tabs,
        RegionNode::default(),
        "a region left out is the app's own"
    );
    assert_eq!(Layout::parse(&layout.to_text()), Ok(layout));

    let toggles =
        Toggles::parse(r#"{ "hidden": { "dock": true, "tool.page": false } }"#).expect("toggles");
    assert_eq!(Toggles::parse(&toggles.to_text()), Ok(toggles));
}

#[test]
fn a_bad_save_keeps_the_last_good_layout_and_says_why() {
    let mut file = LayoutFile::default();
    assert_eq!(file.read(r#"{ "dock": { "hidden": true } }"#), None);
    let good = file.layout().clone();
    assert!(good.dock.hidden);
    for (bad, why) in [
        (r#"{ "dock": { "hidden": tru"#, "line 1"),
        (r#"{ "toolbr": {} }"#, "unknown field `toolbr`"),
        (r#"{ "dock": { "hiden": true } }"#, "unknown field `hiden`"),
        (r#""toolbar""#, "invalid type: string"),
    ] {
        let error = file.read(bad).map_or_default(ToString::to_string);
        assert!(error.contains(why), "{bad}: {error}");
        assert_eq!(file.layout(), &good, "{bad}");
    }
    assert_eq!(file.read("{}"), None);
    assert!(!file.layout().dock.hidden, "a good save replaces it");
    assert!(file.error().is_none());
}
