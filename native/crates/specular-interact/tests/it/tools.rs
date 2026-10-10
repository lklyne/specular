//! The one-shot creation tools: add-shape, add-text, add-sticky and
//! add-page, and the preview each shows under the pointer. The draw tool is
//! in `draw.rs` and page anchoring in `anchoring.rs`.

use specular_core::CssSize;
use specular_doc::{
    Color, ColorPreset, Entity, EntityId, Kind, Page, PageSource, Rect, Shape, ShapeKind, Text,
    TextFont, TextStyle, WidthMode,
};
use specular_interact::{Action, Effect, Key, PlacePreview, TextEdit, Tool, ToolDefaultPatch};
use specular_testkit::{SHIFT, TestApp, assert_doc_snapshot};

/// The entity the last placement left selected.
#[track_caller]
fn placed(app: &TestApp) -> &Entity {
    app.entity(app.selected().unwrap_or("nothing is selected"))
}

/// The entity whose text is being edited.
fn editing(app: &TestApp) -> Option<&EntityId> {
    app.session().editing.as_ref().map(TextEdit::entity)
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

// add-text and add-sticky.

#[test]
fn clicking_with_the_text_tool_places_plain_text_and_starts_editing_it() {
    let mut app = TestApp::empty();
    app.key(Key::Char('t')).click((212.0, 148.0));
    assert_doc_snapshot!(app);
    let entity = placed(&app);
    // An empty text is as wide as its prompt and one line tall.
    assert_eq!(entity.rect, Rect::new(220.0, 140.0, 88.0, 20.0));
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
    assert_eq!(editing(&app), Some(&entity.id));
    assert_eq!(app.session().tool, Tool::Select);
    // A picked ink replaces the theme-following one.
    app.key(Key::Escape)
        .act(Action::SetToolDefault(ToolDefaultPatch::TextColor(Some(
            Color::Preset(ColorPreset::Red),
        ))))
        .key(Key::Char('t'))
        .click((600.0, 400.0));
    assert_eq!(
        text_of(placed(&app)).and_then(|text| text.color.clone()),
        Some(Color::Preset(ColorPreset::Red))
    );
    app.key(Key::Escape).assert_undo_returns_to_start();
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
    assert_eq!(editing(&app), Some(&entity.id));
    app.key(Key::Escape).assert_undo_returns_to_start();
}

#[test]
fn while_a_text_is_edited_letters_are_not_tool_keys_and_escape_ends_the_edit_first() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddSticky).click((100.0, 100.0));
    let id = app.selected().map(str::to_owned);
    app.key(Key::Char('m'))
        .key(Key::Backspace)
        .key(Key::Char('v'));
    assert_eq!(app.session().tool, Tool::Select);
    assert_eq!(app.document().entities().count(), 1, "Backspace is typing");

    app.key(Key::Escape);
    assert_eq!(app.session().editing, None);
    assert_eq!(app.selected(), id.as_deref(), "the selection survives");
    app.key(Key::Escape);
    assert_eq!(app.selected(), None);
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
    // The click is the device's corner. An iPhone SE's screen sits 28 in
    // and 96 down from it.
    assert_eq!(entity.rect, Rect::new(248.0, 236.0, 375.0, 667.0));
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

// The preview.

#[test]
fn an_armed_tool_previews_what_its_click_then_places() {
    // Building the preview from anything but what the click builds lets the
    // two drift: a ghost that is not where, or as big as, what lands.
    for tool in [
        Tool::AddPage,
        Tool::AddText,
        Tool::AddSticky,
        Tool::AddShape,
    ] {
        let mut app = TestApp::empty();
        app.tool(tool);
        assert_eq!(app.app().place_preview(), None, "{tool:?}: no pointer");
        app.pointer_move((105.0, 95.0));
        let Some(PlacePreview::Entity(ghost)) = app.app().place_preview() else {
            panic!("{tool:?} previews nothing");
        };
        assert_eq!(app.document().entities().count(), 0, "{tool:?}");

        app.click((105.0, 95.0));
        let made = placed(&app);
        assert_eq!(
            (made.rect, &made.kind),
            (ghost.rect, &ghost.kind),
            "{tool:?}"
        );
        assert_eq!(
            app.app().place_preview(),
            None,
            "{tool:?}: the tool is spent"
        );
    }

    // A Document has no entity until its file exists, so it is its rect.
    let mut app = TestApp::empty();
    app.tool(Tool::AddDocument).pointer_move((105.0, 95.0));
    assert_eq!(
        app.app().place_preview(),
        Some(PlacePreview::Document(Rect::new(
            100.0, 100.0, 300.0, 300.0
        )))
    );
    // It goes with the pointer, and with the tool.
    app.pointer_leave();
    assert_eq!(app.app().place_preview(), None);
    app.pointer_move((105.0, 95.0)).key(Key::Escape);
    assert_eq!(app.app().place_preview(), None);
}
