//! The comment draft and its composer: where the card sits, what commits or
//! drops the draft, and annotating the selection.

use specular_doc::{AnnotationAnchor, Document, EntityId, Kind, Rect, RegionAnchor};
use specular_interact::{Action, Effect, Key, Tool, selection_metadata};
use specular_testkit::{TestApp, assert_doc_snapshot, document, file, group, inside, page, sticky};

/// The composer's card with `lines` lines of text, in logical pixels: 8 of
/// padding above and below lines of 15.4.
fn card_height(lines: u8) -> f64 {
    f64::from(16.0 + f32::from(lines) * 15.4)
}

/// Two pages with the comment tool armed and a draft open on the empty
/// canvas point (600, 500).
fn drafting() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.tool(Tool::Comment).click((600.0, 500.0)).take_effects();
    app
}

fn texts(app: &TestApp) -> Vec<&str> {
    (app.document().annotations().iter())
        .map(|annotation| annotation.text.as_str())
        .collect()
}

#[test]
fn the_composer_sits_beside_a_point_under_a_region_and_under_an_element() {
    let mut app = drafting();
    let card = |app: &TestApp| app.app().comment_composer();
    assert_eq!(
        card(&app),
        Some(Rect::new(614.0, 487.0, 260.0, card_height(1)))
    );
    // Centred under the region, 8 below it.
    app.drag((550.0, 500.0), (650.0, 560.0));
    assert_eq!(
        card(&app),
        Some(Rect::new(470.0, 568.0, 260.0, card_height(1)))
    );
    // From the left edge of the element's box, 8 below it. The box is in
    // the page's pixels and the page is at (100, 100).
    app.click((200.0, 200.0));
    app.answer_element(specular_core::synthetic_element_at(
        specular_core::CssSize::new(400, 300),
        glam::Vec2::splat(100.0),
    ));
    assert_eq!(
        card(&app),
        Some(Rect::new(100.0, 252.0, 260.0, card_height(1)))
    );
    app.key(Key::Escape);
    assert_eq!(card(&app), None);
}

#[test]
fn the_composer_keeps_its_size_on_screen_at_any_zoom() {
    let mut app = TestApp::with_pages(1);
    app.zoom(2.0).tool(Tool::Comment).click((1200.0, 900.0));
    // The point is (600, 450) on the canvas, and the card 14 right of it and
    // 13 above on screen.
    assert_eq!(
        app.app().comment_composer(),
        Some(Rect::new(607.0, 443.5, 130.0, card_height(1) / 2.0))
    );
    assert_eq!(
        app.app().caret_rect().map(|caret| (caret.x, caret.y)),
        Some((612.0, 447.5)),
        "the text starts 10 in and 8 down"
    );
    let spec = app.app().edit_frame().map(|frame| frame.spec);
    assert_eq!(
        spec.map(|spec| (spec.size, spec.line_height, spec.wrap_width)),
        Some((5.5, 7.7, Some(120.0))),
        "11 px text on 15.4 px lines wrapped at 240 px, as they are on screen"
    );
}

#[test]
fn escape_drops_the_draft_and_leaves_the_comment_tool_armed() {
    let mut app = drafting();
    app.type_text("never mind").key(Key::Escape);
    assert_eq!(
        (
            app.app().comment_draft(),
            app.app().text_edit(),
            texts(&app),
            app.app().can_undo(),
            app.session().tool
        ),
        (None, None, vec![], false, Tool::Comment)
    );
    app.key(Key::Escape);
    assert_eq!(app.session().tool, Tool::Select);
}

#[test]
fn a_draft_with_no_text_commits_nothing_and_leaves_no_undo_step() {
    let mut app = drafting();
    app.key(Key::Enter);
    assert_eq!((app.app().comment_draft(), texts(&app)), (None, vec![]));
    app.click((600.0, 500.0)).type_text("   ").key(Key::Enter);
    assert_eq!(
        (
            texts(&app),
            app.app().can_undo(),
            app.app().focused_comment()
        ),
        (vec![], false, None)
    );
}

#[test]
fn a_press_elsewhere_commits_what_was_written_and_starts_the_next_comment() {
    let mut app = drafting();
    app.type_text("  one ").press((900.0, 700.0));
    assert_eq!(
        texts(&app),
        ["one"],
        "trimmed, and in the document at the press"
    );
    app.release();
    assert_eq!(
        app.app().comment_draft().map(|draft| &draft.anchor),
        Some(&AnnotationAnchor::Canvas {
            canvas_x: 900.0,
            canvas_y: 700.0
        })
    );
    assert_eq!(
        app.app().focused_comment(),
        None,
        "the comment just made does not keep a focus ring under the next draft"
    );
    app.key(Key::Escape).assert_undo_returns_to_start();
}

#[test]
fn switching_tool_commits_the_draft_and_a_new_document_drops_it() {
    let mut app = drafting();
    app.type_text("kept").tool(Tool::Select);
    assert_eq!(texts(&app), ["kept"]);
    app.assert_undo_returns_to_start();

    // Away from the comment just placed: a click on its pill would focus it
    // and open no draft.
    app.tool(Tool::Comment)
        .click((600.0, 620.0))
        .type_text("lost");
    assert!(app.app().comment_draft().is_some() && app.editing_text() == "lost");
    app.take_effects();
    app.open(Document::new());
    assert!(
        app.take_effects().contains(&Effect::SetImeAllowed(false)),
        "the edit ends with the draft"
    );
    assert_eq!(
        (
            app.app().comment_draft(),
            app.app().focused_comment(),
            texts(&app)
        ),
        (None, None, vec![])
    );
}

#[test]
fn annotating_one_selected_page_names_it_as_the_target() {
    let mut app = TestApp::with_pages(2);
    app.tick(86_400_000)
        .select(&["p1"])
        .act(Action::AnnotateSelection);
    let draft = app.comment_draft();
    assert_eq!(
        (&draft.anchor, &draft.page_anchor),
        (
            &AnnotationAnchor::Region(RegionAnchor::Canvas {
                canvas_rect: Rect::new(100.0, 100.0, 400.0, 300.0)
            }),
            &None
        ),
        "the region is the canvas's, though a page fills it"
    );
    app.type_text("tighten this").key(Key::Enter);
    assert_doc_snapshot!(app);
    app.assert_undo_returns_to_start();
    // Two pages: one region spanning both.
    app.select(&["p1", "p2"]).act(Action::AnnotateSelection);
    assert_eq!(
        app.comment_draft().anchor,
        AnnotationAnchor::Region(RegionAnchor::Canvas {
            canvas_rect: Rect::new(100.0, 100.0, 1000.0, 300.0)
        })
    );
}

#[test]
fn the_target_is_the_one_page_selected_or_else_the_one_file() {
    let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let document = document([
        group("g", rect),
        inside("g", page("in-group", rect)),
        page("p", rect),
        file("f", rect),
        file("f2", rect),
        sticky("s", rect, "note"),
    ]);
    let mut blank_page = page("blank-page", rect);
    if let Kind::Page(fields) = &mut blank_page.kind {
        fields.url.clear();
    }
    let mut blank_file = file("blank-file", rect);
    if let Kind::File(fields) = &mut blank_file.kind {
        fields.file.clear();
    }
    let document = {
        let mut entities: Vec<_> = document.entities().cloned().collect();
        entities.extend([blank_page, blank_file]);
        specular_testkit::document(entities)
    };
    let target = |ids: &[&str]| {
        let ids: Vec<EntityId> = ids.iter().map(|id| EntityId::from(*id)).collect();
        (selection_metadata(&document, &ids).get("selectionTarget")).map(ToString::to_string)
    };
    let page = |id: &str| {
        Some(format!(
            r#"{{"entityId":"{id}","kind":"page","url":"https://example.com/{id}"}}"#
        ))
    };
    assert_eq!(
        target(&["p", "s", "f"]),
        page("p"),
        "a page comes before a file"
    );
    assert_eq!(
        target(&["g"]),
        page("in-group"),
        "a group stands for its page"
    );
    assert_eq!(target(&["g", "p"]), None, "two pages");
    assert_eq!(
        target(&["f", "s"]).as_deref(),
        Some(r#"{"entityId":"f","kind":"file","filePath":"f.png"}"#)
    );
    assert_eq!(target(&["f", "f2"]), None, "two files");
    assert_eq!(target(&["s"]), None, "neither");
    assert_eq!(
        target(&["g", "in-group"]),
        page("in-group"),
        "a page selected with its group is one page"
    );
    assert_eq!(
        target(&["p", "in-group", "f"]),
        None,
        "two pages are not settled by a file"
    );
    assert_eq!(
        target(&["blank-page"]).as_deref(),
        Some(r#"{"entityId":"blank-page","kind":"page"}"#),
        "no address, no url"
    );
    assert_eq!(
        target(&["blank-file"]).as_deref(),
        Some(r#"{"entityId":"blank-file","kind":"file"}"#),
        "no path, no filePath"
    );
}
