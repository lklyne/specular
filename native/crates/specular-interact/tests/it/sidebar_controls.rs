//! What the sidebar model reports besides its rows: whether it is shown,
//! the folds, which rows are open, the glyph and trailing text of a row, the
//! rename field and delete of a canvas, and what a comment row sends.

use specular_doc::{
    Annotation, AnnotationAnchor, Entity, EntityId, FileRef, Kind, PageAnchor, Rect,
};
use specular_interact::{Action, Icon, SidebarAction, SidebarRow, SidebarSection, sidebar};
use specular_testkit::{
    TestApp, comment, document, drawing, file, group, inside, note, page, shape, sticky,
    with_comment,
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
fn a_page_row_wears_the_device_its_width_suggests_and_a_file_row_its_kind() {
    let named = |id: &str, name: &str| Entity {
        kind: Kind::File(FileRef {
            file: name.to_owned(),
            ..FileRef::default()
        }),
        ..file(id, BOX)
    };
    let wide = |id: &str, width: f64| Entity {
        rect: Rect::new(0.0, 0.0, width, 500.0),
        ..page(id, BOX)
    };
    let app = TestApp::from_document(document([
        wide("narrow", 599.0),
        wide("tablet", 600.0),
        wide("tablet-top", 1099.0),
        wide("laptop", 1100.0),
        named("img", "a.PNG"),
        named("vid", "a.mp4"),
        named("web", "a.html"),
        named("other", "a.zip"),
        drawing("ink", BOX),
        group("g", Rect::new(0.0, 0.0, 900.0, 900.0)),
        inside("g", group("inner", Rect::new(0.0, 0.0, 800.0, 800.0))),
        inside("inner", sticky("s1", BOX, "a")),
        inside("inner", sticky("s2", BOX, "b")),
        inside("g", sticky("s3", BOX, "c")),
    ]));
    let model = sidebar(app.app());
    let glyphs = |rows: &[SidebarRow]| rows.iter().map(|row| row.glyph).collect::<Vec<_>>();
    assert_eq!(
        glyphs(&model.pages),
        [Icon::Laptop, Icon::Tablet, Icon::Tablet, Icon::Device],
        "front of the stack first"
    );
    assert_eq!(
        glyphs(&model.notes[1..=5]),
        [
            Icon::PenLine,
            Icon::File,
            Icon::Code,
            Icon::Video,
            Icon::Image
        ]
    );
    assert!(
        model
            .pages
            .iter()
            .all(|row| row.expanded.is_none() && row.toggle.is_none()),
        "a page with nothing hooked to it has nothing to open"
    );
    let outer = &model.notes[0];
    assert_eq!(
        outer.trailing.as_deref(),
        Some("3"),
        "every item inside, at any depth"
    );
    assert_eq!(glyphs(&outer.children), [Icon::StickyNote, Icon::Folder]);
}
