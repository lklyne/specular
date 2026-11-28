//! The layout files (ADR 0055, ADR 0059): what they read and write, a file
//! in the old five-region shape read as the tree it means, where the tree
//! puts the canvas, and a bad save that leaves the last good layout in force.

use std::collections::BTreeMap;

use serde_json::json;
use specular_interact::{
    Appearance, Content, Frame, HiddenWord, Layout, LayoutFile, Overlay, Region, SHIPPED, Shown,
    ThemeFile, Toggles, TrafficLights, place,
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
fn a_root_moves_or_hides_the_window_buttons_borders_the_window_and_rounds_the_controls() {
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
    let moved = read(
        r##""traffic_lights": { "x": 10, "y": 6 }, "padding": [0, 6],
            "window_border": { "width": 1, "color": "#000" },"##,
    );
    let border = moved.look.window_border.as_ref().expect("a window border");
    assert_eq!((border.width, border.color.as_str()), (1.0, "#000"));
    let canvas = (place(
        &moved,
        &Toggles::default(),
        shown(),
        Frame::sized(800.0, 600.0),
    ))
    .into_iter()
    .find(|placed| placed.node.content == Some(Content::Canvas))
    .map(|placed| placed.frame);
    assert_eq!(
        canvas.map(|frame| (frame.x, frame.width)),
        Some((6.0, 788.0)),
        "the root's padding insets the chrome from the window's sides"
    );
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
fn a_theme_sets_the_radius_spacing_lines_and_fonts_once_and_a_layout_goes_over_it() {
    let retro: ThemeFile = serde_json::from_str(
        r##"{ "appearance": "light", "radius": 0, "font": "Geneva", "font_size": 12,
              "spacing": 3, "border_width": 2,
              "colors": { "hover": "#000", "hoverText": "#fff" } }"##,
    )
    .expect("a theme");
    let themes = BTreeMap::from([("retro".to_owned(), retro)]);
    let root = |keys: &str| {
        Layout::parse(&format!(
            r#"{{ "type": "column", "theme": "retro", {keys}
                "children": [{{ "id": "canvas", "type": "block", "content": "canvas" }}] }}"#
        ))
        .expect("a layout")
    };
    let themed = root("").look.resolve(Appearance::Dark, &themes);
    assert_eq!(
        (themed.radius, themed.font.as_deref(), themed.font_size),
        (Some(0.0), Some("Geneva"), Some(12.0))
    );
    assert_eq!(
        (themed.spacing, themed.border_width),
        (Some(3.0), Some(2.0))
    );
    assert_eq!(themed.appearance, Some(Appearance::Light));
    assert_eq!(
        themed.colors.get("hoverText").map(String::as_str),
        Some("#fff")
    );
    let over = root(r#""font": "Monaco", "radius": 2, "border_width": 1,"#)
        .look
        .resolve(Appearance::Light, &themes);
    assert_eq!(
        (over.radius, over.font.as_deref(), over.font_size),
        (Some(2.0), Some("Monaco"), Some(12.0))
    );
    assert_eq!((over.spacing, over.border_width), (Some(3.0), Some(1.0)));
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
