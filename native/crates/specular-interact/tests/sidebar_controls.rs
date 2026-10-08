//! What the sidebar model reports besides its rows: whether it is shown,
//! the folds, which rows are open, the glyph and trailing text of a row, the
//! rename field and delete of a canvas, and what a comment row sends.

use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, Entity, EntityId, PageAnchor, Rect,
};
use specular_interact::{
    Action, CanvasAction, FieldSubmit, Icon, SIDEBAR_WIDTH, SidebarAction, SidebarRow,
    SidebarSection, sidebar,
};
use specular_testkit::{
    TestApp, comment, document, group, inside, note, page, shape, sticky, with_comment,
};

const BOX: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 200.0,
};

/// `entity` hooked to `page` as it was on the document at `url`.
fn hooked(page: &str, url: &str, entity: Entity) -> Entity {
    Entity {
        anchor: Some(PageAnchor {
            page_url: Some(url.to_owned()),
            ..PageAnchor::new(EntityId::from(page))
        }),
        ..entity
    }
}

/// A comment bound to `page` as it was on the document at `url`.
fn page_comment(id: &str, page: &str, url: &str, text: &str, at: &str) -> Annotation {
    let anchor = AnnotationAnchor::Page {
        page_id: EntityId::from(page),
        offset_x: 10.0,
        offset_y: 10.0,
    };
    Annotation {
        page_anchor: Some(PageAnchor {
            page_url: Some(url.to_owned()),
            ..PageAnchor::new(EntityId::from(page))
        }),
        created_at: at.to_owned(),
        ..comment(id, anchor, text)
    }
}

#[test]
fn the_sidebar_starts_hidden_and_the_toggle_action_reports_and_flips_it() {
    let mut app = TestApp::with_entities([page("p1", BOX)]);
    let model = sidebar(app.app());
    assert!(!model.visible);
    assert_eq!(model.toggle, Action::Sidebar(SidebarAction::Toggle));
    assert_eq!(app.app().covered_left(), 0.0);
    app.act(model.toggle);
    assert!(sidebar(app.app()).visible);
    assert_eq!(app.app().covered_left(), SIDEBAR_WIDTH);
}

#[test]
fn a_section_reports_whether_it_is_folded_and_the_head_flips_it() {
    let mut app = TestApp::with_space([("Home", document([sticky("s1", BOX, "hi")]))]);
    let model = sidebar(app.app());
    assert_eq!(model.canvases_head.title, "Canvases");
    assert!(!model.notes_head.folded);
    app.act(model.canvases_head.toggle.clone());
    app.act(model.notes_head.toggle.clone());
    let model = sidebar(app.app());
    assert!(model.canvases_head.folded && model.notes_head.folded);
    assert!(!model.pages_head.folded);
    assert_eq!(
        model.canvases_head.title, "Home",
        "a folded list is titled by the canvas shown"
    );
    app.act(model.notes_head.toggle.clone());
    assert!(!sidebar(app.app()).notes_head.folded);
}

#[test]
fn a_group_starts_closed_a_page_open_and_each_toggle_flips_its_own_row() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(0.0, 0.0, 500.0, 500.0)),
        inside("g", sticky("s1", BOX, "in the group")),
        page("p1", BOX),
        hooked(
            "p1",
            "https://example.com/p1",
            sticky("s2", BOX, "on the page"),
        ),
    ]);
    let model = sidebar(app.app());
    let (group_row, page_row) = (&model.notes[0], &model.pages[0]);
    assert_eq!(
        (group_row.expanded, page_row.expanded),
        (Some(false), Some(true))
    );
    assert_eq!(group_row.glyph, specular_interact::Icon::Folder);
    assert_eq!(model.notes[0].id.as_str(), "sidebar.notes.g");
    app.act(
        group_row
            .toggle
            .clone()
            .expect("a group with a member toggles"),
    );
    app.act(
        page_row
            .toggle
            .clone()
            .expect("a page with an item toggles"),
    );
    let model = sidebar(app.app());
    assert_eq!(model.notes[0].expanded, Some(true));
    assert_eq!(model.notes[0].glyph, specular_interact::Icon::FolderOpen);
    assert_eq!(model.pages[0].expanded, Some(false));
    assert!(
        model.notes[0].children[0].toggle.is_none(),
        "a leaf has nothing to open"
    );
}

#[test]
fn a_group_in_both_sections_has_a_row_and_a_name_in_each_and_they_open_apart() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(0.0, 0.0, 500.0, 500.0)),
        inside("g", sticky("s1", BOX, "note")),
        inside("g", page("p1", BOX)),
    ]);
    let model = sidebar(app.app());
    assert_eq!(model.notes[0].id.as_str(), "sidebar.notes.g");
    assert_eq!(model.pages[0].id.as_str(), "sidebar.pages.g");
    app.act(model.notes[0].toggle.clone().expect("toggles"));
    let model = sidebar(app.app());
    assert_eq!(
        (model.notes[0].expanded, model.pages[0].expanded),
        (Some(true), Some(false))
    );
}

#[test]
fn a_row_ends_in_what_it_counts_or_measures_and_shows_the_glyph_of_its_kind() {
    let url = "https://example.com/p1";
    let entities = with_comment(
        document([
            group("g", Rect::new(0.0, 0.0, 900.0, 900.0)),
            inside("g", sticky("s1", BOX, "a")),
            inside("g", sticky("s2", BOX, "b")),
            Entity {
                rect: Rect::new(0.0, 0.0, 375.0, 667.0),
                ..page("p1", BOX)
            },
            note("d1", BOX, "plan.md"),
            shape("r1", BOX),
        ]),
        page_comment("c1", "p1", url, "tighten this", "2026-01-01T00:00:00.000Z"),
    );
    let mut app = TestApp::from_document(entities);
    app.act(Action::Sidebar(SidebarAction::Section(
        SidebarSection::Canvases,
    )));
    let model = sidebar(app.app());
    let read = |row: &SidebarRow| (row.glyph, row.trailing.clone());
    assert_eq!(
        read(&model.notes[0]),
        (Icon::Shape(specular_doc::ShapeKind::Rectangle), None)
    );
    assert_eq!(read(&model.notes[1]), (Icon::FileText, None));
    assert_eq!(read(&model.notes[2]), (Icon::Folder, Some("2".to_owned())));
    assert_eq!(
        read(&model.pages[0]),
        (Icon::Device, Some("375\u{d7}667".to_owned())),
        "a narrow page is drawn as a phone"
    );
    assert_eq!(
        read(&model.pages[0].children[0]),
        (Icon::MessageSquare, None)
    );
}

#[test]
fn a_canvas_row_carries_its_rename_field_and_its_delete() {
    let app = TestApp::with_space([
        ("Home", document([sticky("s1", BOX, "hi")])),
        ("Notes", document([])),
    ]);
    let row = &sidebar(app.app()).canvases[1];
    let id = app.canvas_id("Notes");
    assert_eq!(row.control.as_str(), "sidebar.canvas.tab_2");
    assert_eq!(row.rename.id.as_str(), "sidebar.canvas.tab_2.name");
    assert_eq!(row.rename.value, "Notes");
    assert_eq!(row.rename.submit, FieldSubmit::CanvasName(id.clone()));
    assert_eq!(row.delete, Action::Canvas(CanvasAction::Delete(Some(id))));
    assert_eq!(
        sidebar(app.app()).add_canvas,
        Action::Canvas(CanvasAction::New)
    );
}

#[test]
fn a_comment_row_focuses_the_comment_and_brings_it_into_view() {
    let url = "https://example.com/p1";
    let far = Entity {
        rect: Rect::new(3000.0, 2000.0, 400.0, 300.0),
        ..page("p1", BOX)
    };
    let entities = with_comment(
        document([far]),
        page_comment("c1", "p1", url, "tighten this", "2026-01-01T00:00:00.000Z"),
    );
    let mut app = TestApp::from_document(entities);
    app.viewport((1600.0, 1000.0));
    let row = sidebar(app.app()).pages[0].children[0].clone();
    assert_eq!(row.action, Action::RevealComment(AnnotationId::from("c1")));
    app.act(row.action);
    assert_eq!(app.app().focused_comment(), Some(&AnnotationId::from("c1")));
    // The page is centred, the comment being on the page.
    let on_screen = app
        .session()
        .camera
        .world_to_screen(glam::Vec2::new(3200.0, 2150.0));
    assert_eq!(on_screen, glam::Vec2::new(800.0, 500.0));
}
