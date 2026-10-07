//! The one-shot creation tools: add-shape, add-text, add-sticky and
//! add-page. The draw tool is in `draw.rs` and page anchoring in
//! `anchoring.rs`.

use specular_core::CssSize;
use specular_doc::{
    Color, ColorPreset, Entity, Kind, Page, PageSource, Rect, Shape, ShapeKind, Text, TextFont,
    TextStyle, WidthMode,
};
use specular_interact::{Action, Effect, Key, Tool, ToolDefaultPatch};
use specular_testkit::{SHIFT, TestApp, assert_doc_snapshot};

/// The entity the last placement left selected.
#[track_caller]
fn placed(app: &TestApp) -> &Entity {
    app.entity(app.selected().unwrap_or("nothing is selected"))
}

fn shape_of(entity: &Entity) -> Option<&Shape> {
    match &entity.kind {
        Kind::Shape(shape) => Some(shape),
        _ => None,
    }
}

fn text_of(entity: &Entity) -> Option<&Text> {
    match &entity.kind {
        Kind::Text(text) => Some(text),
        _ => None,
    }
}

fn page_of(entity: &Entity) -> Option<&Page> {
    match &entity.kind {
        Kind::Page(page) => Some(page),
        _ => None,
    }
}

// add-shape.

#[test]
fn dragging_with_the_shape_tool_sizes_a_shape_on_the_grid() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddShape).drag((105.0, 95.0), (251.0, 178.0));
    assert_doc_snapshot!(app);
    assert_eq!(placed(&app).rect, Rect::new(100.0, 100.0, 140.0, 80.0));
    assert_eq!(app.session().tool, Tool::Select, "the tool is one-shot");
    assert_eq!(app.session().editing, None);
    app.undo();
    assert!(!app.app().can_undo(), "the placement was one step");
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn the_shape_being_dragged_out_is_in_the_document_and_named_by_the_app() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddShape).press((100.0, 100.0));
    assert_eq!(app.app().creating(), None, "a press alone makes nothing");
    app.drag_to((200.0, 160.0));
    let live = app.app().creating().cloned().expect("a shape in flight");
    assert_eq!(
        app.rect(live.as_str()),
        Rect::new(100.0, 100.0, 100.0, 60.0)
    );
    assert!(!app.app().can_undo(), "nothing is recorded until release");
    assert_eq!(
        app.take_effects(),
        [Effect::SetCursor(specular_interact::Cursor::Crosshair)]
    );

    // Back under the minimum: there is no shape to show.
    app.drag_to((110.0, 110.0));
    assert_eq!(app.app().creating(), None);
    assert_eq!(app.document().entities().count(), 0);

    app.drag_to((180.0, 200.0)).release();
    assert_eq!(placed(&app).rect, Rect::new(100.0, 100.0, 80.0, 100.0));
    assert_eq!(app.app().creating(), None);
    app.assert_undo_returns_to_start();
}

#[test]
fn clicking_with_the_shape_tool_places_the_default_size_at_the_press() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddShape).click((212.0, 148.0));
    assert_eq!(placed(&app).rect, Rect::new(220.0, 140.0, 160.0, 160.0));
    // A drag too small to size a shape is a click.
    app.tool(Tool::AddShape)
        .drag((600.0, 600.0), (620.0, 700.0));
    assert_eq!(placed(&app).rect, Rect::new(600.0, 600.0, 160.0, 160.0));
    assert_eq!(app.document().entities().count(), 2);
    app.assert_undo_returns_to_start();
}

#[test]
fn shift_keeps_a_dragged_shape_square() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddShape)
        .press((100.0, 100.0))
        .drag_to((200.0, 140.0));
    let live = app.app().creating().cloned().expect("a shape in flight");
    assert_eq!(
        app.rect(live.as_str()),
        Rect::new(100.0, 100.0, 100.0, 40.0)
    );
    // Shift takes effect without the pointer moving.
    app.hold(SHIFT).key_down(Key::Other);
    assert_eq!(
        app.rect(live.as_str()),
        Rect::new(100.0, 100.0, 100.0, 100.0)
    );
    app.release().let_go();
    assert_eq!(placed(&app).rect, Rect::new(100.0, 100.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_shape_takes_its_kind_color_and_stroke_width_from_the_tool_defaults() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddShape).click((0.0, 0.0));
    assert_eq!(
        shape_of(placed(&app)),
        Some(&Shape {
            color: Some(Color::Preset(ColorPreset::Red)),
            stroke_width: Some(2.0),
            text_size: Some(14.0),
            ..Shape::new(ShapeKind::Rectangle)
        })
    );

    app.act(Action::SetToolDefault(ToolDefaultPatch::ShapeColor(
        Color::Preset(ColorPreset::Green),
    )))
    .act(Action::SetToolDefault(ToolDefaultPatch::ShapeStrokeWidth(
        4.0,
    )))
    .chord(SHIFT, Key::Char('r'))
    .drag((400.0, 0.0), (500.0, 100.0));
    assert_eq!(
        shape_of(placed(&app)),
        Some(&Shape {
            color: Some(Color::Preset(ColorPreset::Green)),
            stroke_width: Some(4.0),
            text_size: Some(14.0),
            ..Shape::new(ShapeKind::Diamond)
        })
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_clicked_pill_is_placed_wide() {
    let mut app = TestApp::empty();
    app.act(Action::SetToolVariant(ToolDefaultPatch::ShapeKind(
        ShapeKind::Pill,
    )))
    .click((40.0, 40.0));
    assert_eq!(placed(&app).rect, Rect::new(40.0, 40.0, 200.0, 88.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn escape_mid_drag_takes_the_shape_back_and_records_nothing() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddShape)
        .press((100.0, 100.0))
        .drag_to((300.0, 300.0))
        .key(Key::Escape);
    assert_eq!(app.document().entities().count(), 0);
    assert_eq!(app.session().gesture, None);
    assert_eq!(app.session().tool, Tool::Select);
    assert!(!app.app().can_undo());
    app.release();
    assert_eq!(app.document().entities().count(), 0);
    app.assert_undo_returns_to_start();
}

// add-text and add-sticky.

#[test]
fn clicking_with_the_text_tool_places_plain_text_and_starts_editing_it() {
    let mut app = TestApp::empty();
    app.key(Key::Char('t')).click((212.0, 148.0));
    assert_doc_snapshot!(app);
    let entity = placed(&app);
    assert_eq!(entity.rect, Rect::new(220.0, 140.0, 200.0, 200.0));
    assert_eq!(
        text_of(entity),
        Some(&Text {
            text: String::new(),
            color: Some(Color::Neutral),
            style: Some(TextStyle::Plain),
            width_mode: Some(WidthMode::Auto),
            size: Some(14.0),
            font: Some(TextFont::Sans),
        })
    );
    assert_eq!(app.session().editing.as_ref(), Some(&entity.id));
    assert_eq!(app.session().tool, Tool::Select);
    app.assert_undo_returns_to_start();
}

#[test]
fn clicking_with_the_sticky_tool_places_a_sticky_and_starts_editing_it() {
    let mut app = TestApp::empty();
    app.act(Action::SetToolDefault(ToolDefaultPatch::StickyFont(
        TextFont::Hand,
    )))
    .key(Key::Char('s'))
    // Where the pointer goes before the release does not matter.
    .drag((100.0, 100.0), (400.0, 400.0));
    let entity = placed(&app);
    assert_eq!(entity.rect, Rect::new(100.0, 100.0, 200.0, 200.0));
    assert_eq!(
        text_of(entity),
        Some(&Text {
            text: String::new(),
            color: Some(Color::Preset(ColorPreset::Yellow)),
            style: Some(TextStyle::Sticky),
            width_mode: Some(WidthMode::Fixed),
            size: Some(14.0),
            font: Some(TextFont::Hand),
        })
    );
    assert_eq!(app.session().editing.as_ref(), Some(&entity.id));
    app.assert_undo_returns_to_start();
}

#[test]
fn plain_text_takes_a_picked_color() {
    let mut app = TestApp::empty();
    app.act(Action::SetToolDefault(ToolDefaultPatch::TextColor(Some(
        Color::Preset(ColorPreset::Purple),
    ))))
    .tool(Tool::AddText)
    .click((0.0, 0.0));
    assert_eq!(
        text_of(placed(&app)).and_then(|text| text.color.as_ref()),
        Some(&Color::Preset(ColorPreset::Purple))
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn while_a_text_is_edited_letters_are_not_tool_keys_and_escape_ends_the_edit_first() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddSticky).click((100.0, 100.0));
    let id = app.selected().map(str::to_owned);
    app.key(Key::Char('m')).key(Key::Backspace);
    assert_eq!(app.session().tool, Tool::Select);
    assert_eq!(app.document().entities().count(), 1, "Backspace is typing");

    app.key(Key::Escape);
    assert_eq!(app.session().editing, None);
    assert_eq!(app.selected(), id.as_deref(), "the selection survives");
    app.key(Key::Escape);
    assert_eq!(app.selected(), None);
    app.assert_undo_returns_to_start();
}

#[test]
fn undo_still_works_while_a_text_is_edited_and_ends_the_edit_with_the_entity() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddText).click((100.0, 100.0));
    app.chord(specular_testkit::CMD, Key::Char('z'));
    assert_eq!(app.document().entities().count(), 0);
    assert_eq!(app.session().editing, None);
    app.assert_undo_returns_to_start();
}

#[test]
fn selecting_something_else_ends_the_edit() {
    let mut app = TestApp::with_pages(1);
    app.tool(Tool::AddSticky).click((700.0, 100.0));
    assert!(app.session().editing.is_some());
    app.click((300.0, 250.0));
    assert_eq!(app.selected(), Some("p1"));
    assert_eq!(app.session().editing, None);
    app.assert_undo_returns_to_start();
}

// add-page.

#[test]
fn clicking_with_the_page_tool_places_a_blank_page_and_hosts_it() {
    let mut app = TestApp::empty();
    app.key(Key::Char('p')).take_effects();
    app.click((212.0, 148.0));
    assert_doc_snapshot!(app);
    let entity = placed(&app);
    let id = entity.id.clone();
    assert_eq!(entity.rect, Rect::new(220.0, 140.0, 375.0, 667.0));
    assert_eq!(
        page_of(entity).map(|page| (page.url.as_str(), page.preset_index, page.source)),
        Some(("about:blank", Some(0), Some(PageSource::Manual)))
    );
    assert!(app.effects().contains(&Effect::CreatePage {
        page: id.clone(),
        url: "about:blank".to_owned(),
        viewport: CssSize::new(375, 667),
    }));
    assert_eq!(app.session().tool, Tool::Select);
    assert_eq!(app.session().editing, None);

    app.take_effects();
    app.undo();
    assert!(app.effects().contains(&Effect::ClosePage(id)));
    app.redo().assert_undo_returns_to_start();
}
