//! The layout files (ADR 0055, ADR 0059): what they read and write, a file
//! in the old five-region shape read as the tree it means, where the tree
//! puts the canvas, and a bad save that leaves the last good layout in force.

use serde_json::json;
use specular_interact::{
    Content, Frame, HiddenWord, Layout, LayoutFile, Overlay, Region, SHIPPED, Shown, Toggles,
    TrafficLights, place,
};

#[test]
fn every_shipped_layout_reads_back_as_written() {
    for (name, text) in SHIPPED {
        let layout = Layout::parse(text).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            Layout::parse(&layout.to_text()),
            Ok(layout.clone()),
            "{name}"
        );
    }
    let toggles =
        Toggles::parse(r#"{ "hidden": { "dock": true, "tool.page": false } }"#).expect("toggles");
    assert_eq!(Toggles::parse(&toggles.to_text()), Ok(toggles));
}

#[test]
fn a_root_moves_or_hides_the_window_buttons_and_rounds_the_controls() {
    let canvas = r#""children": [{ "id": "canvas", "type": "block", "content": "canvas" }]"#;
    let read = |root: &str| {
        Layout::parse(&format!(r#"{{ "type": "column", {root} {canvas} }}"#)).expect("a layout")
    };
    let hidden = read(r#""traffic_lights": "hidden", "radius": 0,"#);
    assert_eq!(
        hidden.look.traffic_lights,
        TrafficLights::Hidden(HiddenWord::Hidden)
    );
    assert_eq!(hidden.look.radius, Some(0.0));
    let moved = read(r#""traffic_lights": { "x": 10, "y": 6 },"#);
    assert_eq!(
        moved.look.traffic_lights,
        TrafficLights::At { x: 10.0, y: 6.0 }
    );
    let native = read("");
    assert!(native.look.traffic_lights.is_native());
    for layout in [hidden, moved, native] {
        assert_eq!(Layout::parse(&layout.to_text()), Ok(layout.clone()));
    }
}

#[test]
fn a_region_shaped_file_reads_as_the_tree_it_means() {
    let text = r#"{
        "toolbar": { "mod": "my-tools", "props": { "hidden": ["tool.inspect"], "size": [1, "x"] } },
        "sidebar": { "hidden": true },
        "toast": { "mod": "toast" }
    }"#;
    let layout = Layout::parse(text).expect("a layout");
    assert_eq!(layout.fill(Region::Toolbar), Some("my-tools"));
    let toolbar = layout.node("toolbar").and_then(|node| node.mod_content());
    assert_eq!(
        toolbar.map(|(_, props)| props["size"].clone()),
        Some(json!([1, "x"]))
    );
    assert!(layout.hides("sidebar") && layout.hides("tool.inspect"));
    assert!(!layout.hides("tabs") && !layout.hides("tool.select"));
    assert_eq!(
        layout.fill(Region::Tabs),
        None,
        "a region left out is the app's own"
    );
    assert_eq!(
        layout.overlay(Overlay::Toast).fill.as_deref(),
        Some("toast")
    );
    assert_eq!(Layout::parse(&layout.to_text()), Ok(layout));
}

/// The window's own chrome, the sidebar open, the tools docked, the right
/// panel 300 wide.
fn shown() -> Shown {
    Shown {
        covers: true,
        sidebar: true,
        tools_docked: true,
        item_dock: false,
        chat: Some(300.0),
    }
}

#[test]
fn the_canvas_is_where_the_tree_puts_it() {
    let classic = Layout::classic();
    let right_sidebar = r#"{ "type": "column", "children": [
        { "id": "tabs", "type": "block", "size": { "px": 38 }, "content": { "panel": "tabs" } },
        { "type": "row", "gap": 4, "padding": 8, "children": [
            { "type": "block", "size": { "weight": 2 }, "content": "canvas" },
            { "id": "sidebar", "type": "block", "size": { "px": 392 }, "content": { "mod": "sidebar" } } ] },
        { "id": "status", "type": "block", "size": { "px": 24 }, "content": { "mod": "status" } } ] }"#;
    let right_sidebar = Layout::parse(right_sidebar).expect("a layout");
    let hidden_tabs = Toggles::parse(r#"{ "hidden": { "tabs": true } }"#).expect("toggles");
    let closed = Shown {
        sidebar: false,
        tools_docked: false,
        chat: None,
        ..shown()
    };
    let window = Frame::sized(1200.0, 800.0);
    for (layout, toggles, shown, want) in [
        (
            &classic,
            Toggles::default(),
            shown(),
            Frame {
                x: 292.0,
                y: 38.0,
                width: 608.0,
                height: 762.0,
            },
        ),
        (
            &classic,
            Toggles::default(),
            closed,
            Frame {
                x: 0.0,
                y: 38.0,
                width: 1200.0,
                height: 762.0,
            },
        ),
        (
            &classic,
            hidden_tabs.clone(),
            shown(),
            Frame {
                x: 292.0,
                y: 0.0,
                width: 608.0,
                height: 800.0,
            },
        ),
        (
            &right_sidebar,
            Toggles::default(),
            shown(),
            Frame {
                x: 8.0,
                y: 46.0,
                width: 788.0,
                height: 722.0,
            },
        ),
        (
            &right_sidebar,
            Toggles::default(),
            closed,
            Frame {
                x: 8.0,
                y: 46.0,
                width: 1184.0,
                height: 722.0,
            },
        ),
    ] {
        let canvas = (place(layout, &toggles, shown, window).into_iter())
            .find(|placed| placed.node.content == Some(Content::Canvas))
            .map(|placed| placed.frame);
        assert_eq!(canvas, Some(want), "{shown:?} {toggles:?}");
    }
}

#[test]
fn a_bad_save_keeps_the_last_good_layout_and_says_why() {
    let mut file = LayoutFile::default();
    assert_eq!(file.read(r#"{ "dock": { "hidden": true } }"#), None);
    let good = file.layout().clone();
    assert!(good.hides("dock"));
    for (bad, why) in [
        (r#"{ "dock": { "hidden": tru"#, "line 1"),
        (r#"{ "toolbr": {} }"#, "unknown field `toolbr`"),
        (r#"{ "dock": { "hiden": true } }"#, "unknown field `hiden`"),
        (r#""toolbar""#, "a layout is a JSON object"),
        (
            r#"{ "type": "row", "children": [] }"#,
            "one block with \"content\": \"canvas\"",
        ),
        (
            r#"{ "type": "row", "children": [{ "type": "block", "content": "canvas" }, { "type": "block", "content": { "panel": "dock" } }] }"#,
            "no panel `dock`",
        ),
        (
            r#"{ "type": "row", "children": [{ "id": "a", "type": "block", "content": "canvas" }, { "id": "a", "type": "block" }] }"#,
            "two nodes have the id `a`",
        ),
    ] {
        let error = file.read(bad).map_or_default(ToString::to_string);
        assert!(error.contains(why), "{bad}: {error}");
        assert_eq!(file.layout(), &good, "{bad}");
    }
    assert_eq!(file.read("{}"), None);
    assert!(!file.layout().hides("dock"), "a good save replaces it");
    assert!(file.error().is_none());
}
