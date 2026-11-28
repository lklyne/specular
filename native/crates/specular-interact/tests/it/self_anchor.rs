//! A page never rides its own scroll. A web mod item is a page, and one
//! saved hooked to itself slid with its own document and was cut and faded
//! to its own frame.

use specular_doc::{EntityId, PageAnchor, Rect};
use specular_interact::{PageNotice, seen, shown_rect};
use specular_testkit::{TestApp, page};

#[test]
fn a_page_hooked_to_itself_stays_where_it_is_when_it_scrolls() {
    let rect = Rect::new(100.0, 100.0, 400.0, 300.0);
    let mut hooked = page("p1", rect);
    hooked.anchor = Some(PageAnchor {
        scroll_x: Some(0.0),
        scroll_y: Some(0.0),
        ..PageAnchor::new(EntityId::from("p1"))
    });
    let mut app = TestApp::with_entities([hooked]);
    app.page_reports("p1", PageNotice::Scrolled { x: 0.0, y: 240.0 });

    let id = EntityId::from("p1");
    let entity = app.document().entity(&id);
    assert_eq!(entity.and_then(|e| shown_rect(app.app(), e)), Some(rect));
    let clip = entity
        .and_then(|e| seen(app.app(), e))
        .map(|seen| seen.clip);
    assert_eq!(clip, Some(None), "a page is not cut to its own frame");
}
