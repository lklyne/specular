//! A control's action, taken out of the model and run, changes the document
//! or the tool defaults, and the next model shows it.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::{Color, ColorPreset, Kind, Rect, ShapeKind, TextStyle};
use specular_interact::{
    Action, Control, ControlId, DropdownSection, Effect, Format, PopupModel, Property, Tool,
    ToolbarSection, popup_for, toolbar,
};
use specular_testkit::{TestApp, document, plain_text, shape, sticky, with_edge};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 100.0, 200.0, 100.0);
const RED: Color = Color::Preset(ColorPreset::Red);

fn popup(app: &TestApp) -> PopupModel {
    popup_for(app.app()).unwrap_or_else(|| panic!("no popup"))
}

/// Runs the action of the control `id` in the popup, as a renderer would on
/// a press.
fn press(app: &mut TestApp, id: &str) {
    let id = ControlId::from(id.to_owned());
    let Some(action) = popup(app).action(&id) else {
        panic!("no control {id}");
    };
    app.act(action);
}

/// Whether the toggle, option or swatch `id` is on.
fn is_on(model: &PopupModel, id: &str) -> bool {
    fn find(controls: &[Control], id: &str) -> Option<bool> {
        for control in controls {
            let found = match control {
                Control::Toggle(toggle) => (toggle.id.as_str() == id).then_some(toggle.on),
                Control::Swatches(swatches) => (swatches.options.iter())
                    .find(|swatch| swatch.id.as_str() == id)
                    .map(|swatch| swatch.selected),
                Control::Dropdown(dropdown) => {
                    dropdown.content.iter().find_map(|section| match section {
                        DropdownSection::Options { options, .. } => (options.iter())
                            .find(|option| option.id.as_str() == id)
                            .map(|option| option.selected),
                        DropdownSection::Controls(row) => find(row, id),
                    })
                }
                Control::Button(_) | Control::Stepper(_) | Control::Separator => None,
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }
    find(&model.controls, id).unwrap_or_else(|| panic!("no control {id}"))
}

#[test]
fn a_swatch_sets_the_color_and_the_next_model_selects_it() {
    let mut app = TestApp::with_entities([sticky("a", A, "one"), sticky("b", B, "two")]);
    app.select(&["a", "b"]);
    assert!(!is_on(&popup(&app), "text.color.swatches.red"));
    press(&mut app, "text.color.swatches.red");
    assert!(is_on(&popup(&app), "text.color.swatches.red"));
    assert!(!is_on(&popup(&app), "text.color.swatches.blue"));
    for id in ["a", "b"] {
        let Kind::Text(text) = &app.entity(id).kind else {
            panic!("text");
        };
        assert_eq!(text.color, Some(RED));
    }
    app.assert_undo_returns_to_start();
}

#[test]
fn every_swatch_carries_the_action_that_sets_its_own_color() {
    let mut app = TestApp::with_entities([shape("s", A)]);
    app.select(&["s"]);
    let model = popup(&app);
    let mut checked = 0;
    for control in &model.controls {
        let Control::Dropdown(dropdown) = control else {
            continue;
        };
        for section in &dropdown.content {
            let DropdownSection::Controls(row) = section else {
                continue;
            };
            for control in row {
                let Control::Swatches(swatches) = control else {
                    continue;
                };
                for swatch in &swatches.options {
                    let expected = match (&swatch.color, dropdown.id.as_str()) {
                        (Some(color), "shape.color") => Property::Color(color.clone()),
                        (Some(color), _) => Property::BorderColor(color.clone()),
                        (None, _) => Property::FillStyle(specular_doc::FillStyle::None),
                    };
                    assert_eq!(
                        swatch.action,
                        Action::SetProperty(expected),
                        "{}",
                        swatch.id
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 8 + 1 + 8, "fill with clear, then border");
}

#[test]
fn clearing_a_fill_shows_the_clear_swatch_and_a_color_brings_the_fill_back() {
    let mut app = TestApp::with_entities([shape("s", A)]);
    app.select(&["s"]);
    press(&mut app, "shape.color.swatches.transparent");
    assert!(is_on(&popup(&app), "shape.color.swatches.transparent"));
    assert!(!is_on(&popup(&app), "shape.color.swatches.red"));
    press(&mut app, "shape.color.swatches.blue");
    assert!(!is_on(&popup(&app), "shape.color.swatches.transparent"));
    assert!(is_on(&popup(&app), "shape.color.swatches.blue"));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_toggle_goes_to_the_other_state_and_back() {
    let doc = document([plain_text("a", A, "a"), plain_text("b", B, "b")]);
    let mut app = TestApp::from_document(with_edge(doc, specular_doc::Edge::new("e", "a", "b")));
    app.select(&["e"]);
    assert!(!is_on(&popup(&app), "edge.start"));
    press(&mut app, "edge.start");
    assert!(is_on(&popup(&app), "edge.start"));
    press(&mut app, "edge.start");
    assert!(!is_on(&popup(&app), "edge.start"));
    app.undo().undo().assert_undo_returns_to_start();
}

#[test]
fn the_delete_button_removes_the_edge_and_the_popup_goes() {
    let doc = document([plain_text("a", A, "a"), plain_text("b", B, "b")]);
    let mut app = TestApp::from_document(with_edge(doc, specular_doc::Edge::new("e", "a", "b")));
    app.select(&["e"]);
    press(&mut app, "edge.delete");
    assert!(popup_for(app.app()).is_none());
    assert!(app.document().edges().next().is_none());
    app.assert_undo_returns_to_start();
}

#[test]
fn a_named_size_and_the_stepper_both_set_the_size() {
    let mut app = TestApp::with_entities([shape("s", A)]);
    app.select(&["s"]);
    press(&mut app, "shape.size.32");
    assert!(is_on(&popup(&app), "shape.size.32"));
    press(&mut app, "shape.size.custom.inc");
    assert!(
        !is_on(&popup(&app), "shape.size.32"),
        "33 is not a named size"
    );
    assert_eq!(
        specular_interact::property::read::text_size(app.app()),
        Some(33.0)
    );
}

#[test]
fn a_page_option_resizes_the_page_and_the_toggles_follow() {
    let mut app = TestApp::with_pages(1);
    app.select(&["p1"]);
    press(&mut app, "page.size.1");
    assert!(is_on(&popup(&app), "page.size.1"));
    assert_eq!(app.rect("p1").width, 393.0);
    assert!(!is_on(&popup(&app), "page.frame"));
    press(&mut app, "page.frame");
    assert!(is_on(&popup(&app), "page.frame"));
    press(&mut app, "page.scheme");
    press(&mut app, "page.scheme");
    let Some(Action::SetProperty(Property::ColorScheme(next))) =
        popup(&app).action(&ControlId::new("page.scheme"))
    else {
        panic!("a scheme action");
    };
    assert_eq!(next, None, "light, dark, then back to the system's");
}

#[test]
fn a_text_style_property_is_not_a_control() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    app.select(&["a"]);
    let model = popup(&app);
    assert!(
        model.entries().iter().all(|(_, action)| *action
            != Some(&Action::SetProperty(Property::TextStyle(TextStyle::Plain))))
    );
}

#[test]
fn a_tool_popup_writes_the_defaults_and_asks_for_a_save() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddShape);
    app.take_effects();
    press(&mut app, "shape.kind.pill");
    assert_eq!(app.app().tool_defaults().shape.kind, ShapeKind::Pill);
    let saves = (app.take_effects().into_iter())
        .filter(|effect| matches!(effect, Effect::SaveToolDefaults(_)))
        .count();
    assert_eq!(saves, 1);
    assert!(is_on(&popup(&app), "shape.kind.pill"));
    press(&mut app, "shape.color.green");
    assert!(is_on(&popup(&app), "shape.color.green"));
    assert!(!app.app().can_undo(), "defaults are not in undo");
}

#[test]
fn the_formatting_buttons_format_the_text_being_edited() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    app.double_click((150.0, 150.0));
    press(&mut app, "format.bold");
    assert_eq!(app.editing_text(), "**one**");
    assert_eq!(
        popup(&app).action(&ControlId::new("format").child("bold")),
        Some(Action::Format(Format::Bold))
    );
}

#[test]
fn a_toolbar_button_arms_its_tool_and_a_zoom_level_zooms() {
    let mut app = TestApp::empty();
    let action = toolbar(app.app())
        .action(&ControlId::new("tool").child("sticky"))
        .unwrap_or_else(|| panic!("a sticky button"));
    app.act(action);
    assert_eq!(app.app().session().tool, Tool::AddSticky);
    let model = toolbar(app.app());
    let active: Vec<Tool> = (model.sections.iter())
        .flat_map(|section| match section {
            ToolbarSection::Tools(tools) => tools.as_slice(),
            ToolbarSection::Zoom(_) => &[],
        })
        .filter(|tool| tool.active)
        .map(|tool| tool.tool)
        .collect();
    assert_eq!(active, [Tool::AddSticky]);
    let zoom = model.action(&ControlId::new("zoom").child(50));
    app.act(zoom.unwrap_or_else(|| panic!("a 50% level")));
    assert!((app.app().session().camera.zoom - 0.5).abs() < 1e-6);
}

#[test]
fn control_ids_are_unique_within_a_model() {
    let mut app = TestApp::with_entities([shape("s", A), sticky("t", B, "x")]);
    for ids in [&["s"][..], &["t"][..]] {
        app.select(ids);
        let model = popup(&app);
        let entries = model.entries();
        let mut seen: Vec<&ControlId> = entries.iter().map(|(id, _)| id).collect();
        let total = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), total);
    }
    let model = toolbar(app.app());
    let entries = model.entries();
    let mut seen: Vec<&ControlId> = entries.iter().map(|(id, _)| id).collect();
    let total = seen.len();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), total);
}
