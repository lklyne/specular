//! Two apps on one shared canvas (ADR 0048): what one does by hand reaches
//! the other, and undo takes back only the undoing app's own steps.

use specular_doc::{Command, Document, Kind, Rect};
use specular_interact::panel::{ShareButton, toolbar};
use specular_interact::{
    Action, ApiCall, ApiRun, Effect, Event, Key, LinkScope, ShareAction, ShareEvent, ShareLinks,
};
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

/// What a live room does with its first answer: `state` as it stands.
fn connected(app: &mut TestApp, state: Vec<u8>) {
    let canvas = app.app().space().active().id.clone();
    app.send(Event::Share(ShareEvent::Connected { canvas, state }));
}

fn share(app: &mut TestApp, fresh: bool) -> Vec<Effect> {
    let run = ApiRun::Share {
        link: "https://specular.example/c/doc#t=edit".to_owned(),
        fresh,
    };
    let call = ApiCall {
        ticket: 1,
        canvas: None,
        thread: None,
        run,
    };
    app.send(Event::Api(call));
    app.take_effects()
}

#[test]
fn a_live_room_shares_the_first_canvas_and_a_joiner_gets_it_in_a_new_one() {
    let mut a = TestApp::from_document(document([
        sticky("s1", rect(0.0, 0.0), "one"),
        sticky("s2", rect(400.0, 0.0), "two"),
    ]));
    let effects = share(&mut a, false);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::ShareConnect { link, .. } if link.ends_with("#t=edit"))),
        "{effects:?}"
    );
    // An empty room: this canvas is the shared one.
    connected(&mut a, Vec::new());
    let mut room = sent(&mut a);
    assert_eq!(room.len(), 1, "the whole canvas goes to the room");
    assert!(a.app().is_shared());

    let mut b = TestApp::from_document(document([sticky("mine", rect(0.0, 300.0), "keep")]));
    let mine = b.app().space().active().id.clone();
    share(&mut b, true);
    assert_ne!(b.app().space().active().id, mine, "joining makes a canvas");
    connected(&mut b, room.remove(0));
    b.take_effects();
    assert_eq!(b.document().order(), a.document().order());
    let kept = b.app().canvas_document(&mine).expect("the old canvas");
    assert!(
        kept.entity(&"mine".into()).is_some(),
        "the old canvas is left as it was"
    );

    // B drops off, edits offline, and comes back to a room that moved on.
    a.drag((100.0, 50.0), (100.0, 450.0));
    let missed = sent(&mut a);
    b.drag((500.0, 50.0), (500.0, 450.0));
    sent(&mut b);
    let moved_by_b = b.rect("s2");
    assert_ne!(moved_by_b, rect(400.0, 0.0));
    let mut state_now = missed;
    connected(&mut b, state_now.remove(0));
    let catch_up = sent(&mut b);
    assert_eq!(catch_up.len(), 1, "a reconnect sends everything it holds");
    deliver(&mut a, catch_up);
    exchange(&mut a, &mut b);
    assert_eq!(a.document(), b.document());
    assert_eq!(
        a.rect("s2"),
        moved_by_b,
        "the offline edit reached the room"
    );
    assert_ne!(
        b.rect("s1"),
        rect(0.0, 0.0),
        "and the edit it missed reached it"
    );

    a.send(Event::Api(ApiCall {
        ticket: 2,
        canvas: None,
        thread: None,
        run: ApiRun::Unshare,
    }));
    assert!(
        a.take_effects()
            .iter()
            .any(|effect| matches!(effect, Effect::ShareDisconnect(_)))
    );
    assert!(!a.app().is_shared());
}

fn press_share(app: &mut TestApp, action: ShareAction) -> Vec<Effect> {
    app.send(Event::Action(Action::Share(action)));
    app.take_effects()
}

fn share_button(app: &TestApp) -> ShareButton {
    toolbar(app.app()).share
}

fn access(app: &TestApp) -> String {
    (share_button(app).access.summary.text).map_or_else(String::new, |text| text.to_string())
}

fn copied(effects: &[Effect]) -> Option<&str> {
    effects.iter().find_map(|effect| match effect {
        Effect::WriteClipboard(link) => Some(link.as_str()),
        _ => None,
    })
}

#[test]
fn copy_link_publishes_goes_live_and_copies_the_link_for_the_access_picked() {
    let mut app = TestApp::from_document(document([sticky("s1", rect(0.0, 0.0), "one")]));
    assert_eq!(access(&app), "Only you");
    let canvas = app.app().space().active().id.clone();

    let effects = press_share(&mut app, ShareAction::CopyLink);
    assert!(
        (effects.iter())
            .any(|effect| matches!(effect, Effect::SharePublish { canvas: c, .. } if *c == canvas)),
        "{effects:?}"
    );
    assert!(
        !share_button(&app).copy.enabled,
        "no second publish while one runs"
    );
    assert_eq!(press_share(&mut app, ShareAction::CopyLink), Vec::new());

    let links = ShareLinks {
        edit: "https://s.example/c/d#t=e".to_owned(),
        comment: "https://s.example/c/d#t=c".to_owned(),
        view: "https://s.example/c/d#t=v".to_owned(),
    };
    app.send(Event::Share(ShareEvent::Published {
        canvas: canvas.clone(),
        links,
    }));
    let effects = app.take_effects();
    assert_eq!(
        copied(&effects),
        Some("https://s.example/c/d#t=e"),
        "edit is the default"
    );
    assert!(
        (effects.iter()).any(
            |effect| matches!(effect, Effect::ShareConnect { link, .. } if link.ends_with("#t=e"))
        ),
        "it goes live through the edit link: {effects:?}"
    );
    connected(&mut app, Vec::new());
    app.take_effects();
    assert!(app.app().is_shared());
    assert_eq!(access(&app), "Anyone with the link can edit");

    press_share(&mut app, ShareAction::SetAccess(Some(LinkScope::View)));
    assert_eq!(access(&app), "Anyone with the link can view");
    let effects = press_share(&mut app, ShareAction::CopyLink);
    assert_eq!(copied(&effects), Some("https://s.example/c/d#t=v"));

    let effects = press_share(&mut app, ShareAction::SetAccess(None));
    assert!((effects.iter()).any(|effect| matches!(effect, Effect::ShareDisconnect(_))));
    assert!(!app.app().is_shared());
    assert_eq!(access(&app), "Only you");
}

#[test]
fn picking_who_can_open_the_link_shares_without_copying() {
    let mut app = TestApp::from_document(document([sticky("s1", rect(0.0, 0.0), "one")]));
    let canvas = app.app().space().active().id.clone();
    let effects = press_share(&mut app, ShareAction::SetAccess(Some(LinkScope::Comment)));
    assert!((effects.iter()).any(|effect| matches!(effect, Effect::SharePublish { .. })));
    let links = ShareLinks {
        edit: "https://s.example/c/d#t=e".to_owned(),
        comment: "https://s.example/c/d#t=c".to_owned(),
        view: "https://s.example/c/d#t=v".to_owned(),
    };
    app.send(Event::Share(ShareEvent::Published { canvas, links }));
    assert_eq!(copied(&app.take_effects()), None);
    assert_eq!(access(&app), "Anyone with the link can comment");
}

#[test]
fn a_failed_publish_or_a_refused_room_says_why_and_can_be_tried_again() {
    let mut app = TestApp::from_document(document([sticky("s1", rect(0.0, 0.0), "one")]));
    let canvas = app.app().space().active().id.clone();
    press_share(&mut app, ShareAction::CopyLink);
    app.send(Event::Share(ShareEvent::PublishFailed {
        canvas: canvas.clone(),
        reason: "the share server answered 500".to_owned(),
    }));
    let button = share_button(&app);
    let note = button.note.expect("a reason").to_string();
    assert!(note.contains("500"), "{note}");
    assert!(button.copy.enabled);
    assert_eq!(access(&app), "Only you");

    share(&mut app, false);
    connected(&mut app, Vec::new());
    app.take_effects();
    assert!(app.app().is_shared());
    app.send(Event::Share(ShareEvent::Ended {
        canvas,
        reason: "the link is revoked or wrong".to_owned(),
    }));
    assert!(!app.app().is_shared());
    assert!(share_button(&app).note.is_some());
}
