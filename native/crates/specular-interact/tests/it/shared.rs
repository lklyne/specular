//! Two apps on one shared canvas (ADR 0048): what one does by hand reaches
//! the other, and undo takes back only the undoing app's own steps.

use specular_doc::{Command, Document, Kind, Rect};
use specular_interact::{Action, Effect, Event, Key, ShareEvent};
use specular_testkit::{TestApp, document, page, sticky};

/// Delivers each app's share updates to the other until neither has
/// anything left to send, as a transport would.
fn exchange(a: &mut TestApp, b: &mut TestApp) {
    for round in 0.. {
        let from_a = sent(a);
        let from_b = sent(b);
        if from_a.is_empty() && from_b.is_empty() {
            return;
        }
        assert!(round < 8, "the apps kept sending");
        deliver(b, from_a);
        deliver(a, from_b);
    }
}

fn sent(app: &mut TestApp) -> Vec<Vec<u8>> {
    (app.take_effects().into_iter())
        .filter_map(|effect| match effect {
            Effect::ShareSend { update, .. } => Some(update),
            _ => None,
        })
        .collect()
}

fn deliver(app: &mut TestApp, updates: Vec<Vec<u8>>) {
    for update in updates {
        let canvas = app.app().space().active().id.clone();
        app.send(Event::Share(ShareEvent::Receive { canvas, update }));
    }
}

/// `a` shares its canvas and a new app joins it.
fn shared(document: Document) -> (TestApp, TestApp) {
    let mut a = TestApp::from_document(document);
    a.send(Event::Share(ShareEvent::Start));
    let mut state = sent(&mut a);
    assert_eq!(state.len(), 1, "starting sends the whole canvas once");
    let mut b = TestApp::empty();
    b.send(Event::Share(ShareEvent::Join(state.remove(0))));
    b.take_effects();
    (a, b)
}

fn rect(x: f64, y: f64) -> Rect {
    Rect::new(x, y, 200.0, 100.0)
}

#[test]
fn a_drag_reaches_the_peer_frame_by_frame_and_only_its_maker_can_undo_it() {
    let (mut a, mut b) = shared(document([
        sticky("s1", rect(0.0, 0.0), "one"),
        sticky("s2", rect(400.0, 0.0), "two"),
    ]));
    assert_eq!(a.document().order(), b.document().order());
    assert_eq!(b.rect("s2"), rect(400.0, 0.0));

    a.press((100.0, 50.0)).drag_to((150.0, 50.0));
    exchange(&mut a, &mut b);
    assert_ne!(a.rect("s1"), rect(0.0, 0.0));
    assert_eq!(
        b.rect("s1"),
        a.rect("s1"),
        "the peer sees the drag as it goes"
    );
    a.drag_to((200.0, 250.0)).release();
    b.drag((500.0, 50.0), (500.0, 450.0));
    exchange(&mut a, &mut b);
    let (moved_by_a, moved_by_b) = (a.rect("s1"), b.rect("s2"));
    assert_ne!(moved_by_b, rect(400.0, 0.0));
    assert_eq!(b.document(), a.document());

    // A's undo takes back A's drag and leaves B's.
    a.act(Action::Undo);
    exchange(&mut a, &mut b);
    for app in [&a, &b] {
        assert_eq!(app.rect("s1"), rect(0.0, 0.0));
        assert_eq!(app.rect("s2"), moved_by_b);
    }
    assert!(!a.app().can_undo());
    assert!(b.app().can_undo());

    a.act(Action::Redo);
    exchange(&mut a, &mut b);
    assert_eq!(b.rect("s1"), moved_by_a);
}

#[test]
fn typing_deleting_and_changes_made_outside_a_step_reach_the_peer() {
    let (mut a, mut b) = shared(document([
        sticky("s1", rect(0.0, 0.0), "one"),
        page("p1", Rect::new(400.0, 0.0, 400.0, 300.0)),
    ]));

    b.double_click((100.0, 50.0))
        .type_text("!")
        .key(Key::Escape);
    exchange(&mut a, &mut b);
    let Kind::Text(text) = &a.document().entity(&"s1".into()).expect("s1").kind else {
        panic!("s1 is a text");
    };
    assert!(text.text.ends_with('!'), "{:?}", text.text);

    a.select(&["p1"]).act(Action::Delete);
    exchange(&mut a, &mut b);
    assert!(b.document().entity(&"p1".into()).is_none());
    b.act(Action::Undo);
    exchange(&mut a, &mut b);
    assert!(
        a.document().entity(&"p1".into()).is_none(),
        "B cannot undo A's delete"
    );
    a.act(Action::Undo);
    exchange(&mut a, &mut b);
    assert!(b.document().entity(&"p1".into()).is_some());

    // A document read again from its file, as after an agent wrote it,
    // reaches the peer too: the file is the canvas.
    let mut changed = a.document().clone();
    changed
        .apply(Command::SetRect {
            id: "s1".into(),
            rect: rect(0.0, 600.0),
        })
        .expect("move");
    a.send(Event::DocumentOpened(Box::new(changed)));
    exchange(&mut a, &mut b);
    assert_eq!(b.rect("s1"), rect(0.0, 600.0));
    assert_eq!(b.document(), a.document());
}
