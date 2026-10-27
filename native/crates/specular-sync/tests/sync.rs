//! Peers editing one canvas through `CanvasSync`: they end up with the same
//! document, and undo takes back only the undoing peer's own steps.

#![expect(clippy::expect_used, reason = "a test fails by panicking")]

use specular_doc::{AnnotationAnchor, Command, Document, Edge, EntityId, ItemId, Kind, Rect};
use specular_sync::CanvasSync;
use specular_testkit::{comment, connected, document, group, inside, page, sticky, with_comment};

/// One peer: its document and its shared document, with the selection as
/// the state its undo steps carry.
struct Peer {
    sync: CanvasSync<Vec<EntityId>>,
    doc: Document,
}

impl Peer {
    fn publish(doc: Document) -> Self {
        Self {
            sync: CanvasSync::new(&doc),
            doc,
        }
    }

    fn join(from: &Peer) -> Self {
        let (sync, doc) = CanvasSync::join(&from.sync.encode_state()).expect("join");
        Self { sync, doc }
    }

    #[track_caller]
    fn act(&mut self, command: Command) {
        self.sync
            .apply(&mut self.doc, command)
            .expect("command applies");
    }

    fn rect(&self, id: &str) -> Rect {
        self.doc.entity(&EntityId::from(id)).expect("entity").rect
    }

    fn label(&self, id: &str) -> Option<String> {
        self.doc
            .entity(&EntityId::from(id))
            .expect("entity")
            .label
            .clone()
    }

    fn has(&self, id: &str) -> bool {
        self.doc.entity(&EntityId::from(id)).is_some()
    }
}

/// Delivers every peer's pending updates to every other peer, later peers'
/// first, so that delivery order differs from the order edits were made.
fn exchange(peers: &mut [&mut Peer]) {
    let outgoing: Vec<Option<Vec<u8>>> = peers.iter_mut().map(|p| p.sync.take_outgoing()).collect();
    for (from, update) in outgoing.iter().enumerate().rev() {
        let Some(update) = update else { continue };
        for (to, peer) in peers.iter_mut().enumerate() {
            if to != from {
                peer.sync
                    .apply_remote(update, &mut peer.doc)
                    .expect("update applies");
            }
        }
    }
}

/// The parts of two documents that are shared: everything but the view
/// state in `extra`.
#[track_caller]
fn assert_same(a: &Document, b: &Document) {
    let entities = |d: &Document| d.entities().cloned().collect::<Vec<_>>();
    let edges = |d: &Document| d.edges().cloned().collect::<Vec<_>>();
    let notes = |d: &Document| {
        let mut notes: Vec<(String, String)> = d
            .notes()
            .map(|(f, t)| (f.to_owned(), t.to_owned()))
            .collect();
        notes.sort();
        notes
    };
    assert_eq!(a.order(), b.order(), "stack order");
    assert_eq!(entities(a), entities(b), "entities");
    assert_eq!(edges(a), edges(b), "edges");
    assert_eq!(a.annotations(), b.annotations(), "annotations");
    assert_eq!(notes(a), notes(b), "notes");
}

fn rect(x: f64, y: f64) -> Rect {
    Rect::new(x, y, 200.0, 100.0)
}

fn three_stickies() -> Document {
    document([
        sticky("a", rect(0.0, 0.0), "A"),
        sticky("b", rect(300.0, 0.0), "B"),
        sticky("c", rect(600.0, 0.0), "C"),
    ])
}

#[test]
fn a_joining_peer_gets_the_canvas_exactly() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");
    for path in [
        "native/fixtures/kitchen-sink.canvas",
        "native/crates/specular-doc/tests/fixtures/rich-workspace.canvas",
        "resources/starter-space/Welcome.canvas",
        "native/fixtures/bench/drawings-300.canvas",
        "native/fixtures/bench/edges-200.canvas",
    ] {
        let json = std::fs::read_to_string(format!("{root}/{path}")).expect(path);
        let mut doc = Document::from_canvas_str(&json).expect(path);
        doc.apply(Command::SetNote {
            file: "plan.md".into(),
            text: Some("# Plan\n\nship it 🚀".into()),
        })
        .expect("note");
        let publisher = Peer::publish(doc);
        let joined = Peer::join(&publisher);
        assert_same(&publisher.doc, &joined.doc);
    }
}

#[test]
fn edits_made_at_the_same_time_all_land_and_every_peer_agrees() {
    let start = with_comment(
        connected(three_stickies(), "e1", "a", "b"),
        comment(
            "c1",
            AnnotationAnchor::Canvas {
                canvas_x: 0.0,
                canvas_y: 0.0,
            },
            "hi",
        ),
    );
    let mut ann = Peer::publish(start);
    let mut ben = Peer::join(&ann);
    let mut cat = Peer::join(&ann);

    // Different fields of one sticky, a removal, an insert, an edge and a
    // comment, all before anyone hears from anyone.
    ann.act(Command::SetRect {
        id: "a".into(),
        rect: rect(10.0, 20.0),
    });
    ben.act(Command::SetLabel {
        id: "a".into(),
        label: Some("Ben's".into()),
    });
    ben.act(Command::RemoveEntity("c".into()));
    cat.act(Command::InsertEntity {
        entity: Box::new(sticky("d", rect(0.0, 300.0), "D")),
        at: 1,
    });
    cat.act(Command::InsertEdge {
        edge: Box::new(Edge::new("e2", "b", "d")),
        at: cat.doc.stack_len(),
    });
    ann.act(Command::InsertAnnotation {
        annotation: Box::new(comment(
            "c2",
            AnnotationAnchor::Canvas {
                canvas_x: 5.0,
                canvas_y: 5.0,
            },
            "there",
        )),
        at: 0,
    });
    exchange(&mut [&mut ann, &mut ben, &mut cat]);

    assert_same(&ann.doc, &ben.doc);
    assert_same(&ann.doc, &cat.doc);
    assert_eq!(ann.rect("a"), rect(10.0, 20.0));
    assert_eq!(ann.label("a").as_deref(), Some("Ben's"));
    assert!(!ann.has("c"));
    assert!(ann.has("d"));
    assert_eq!(ann.doc.stack_index(&ItemId::Entity("d".into())), Some(1));
    let comments: Vec<&str> = ann
        .doc
        .annotations()
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(comments, ["c2", "c1"]);

    // Two fields of one kind: the text and the size of one sticky.
    let retyped = |peer: &Peer, change: fn(&mut specular_doc::Text)| {
        let Kind::Text(mut text) = peer.doc.entity(&"b".into()).expect("b").kind.clone() else {
            panic!("b is a text");
        };
        change(&mut text);
        Command::SetKind {
            id: "b".into(),
            kind: Box::new(Kind::Text(text)),
        }
    };
    ann.act(retyped(&ann, |text| text.text = "Ann's".into()));
    ben.act(retyped(&ben, |text| text.size = Some(40.0)));
    exchange(&mut [&mut ann, &mut ben, &mut cat]);
    assert_same(&ann.doc, &cat.doc);
    let Kind::Text(text) = &cat.doc.entity(&"b".into()).expect("b").kind else {
        panic!("b is a text");
    };
    assert_eq!((text.text.as_str(), text.size), ("Ann's", Some(40.0)));

    // The same field changed twice: one value wins, the same one everywhere.
    ann.act(Command::SetRect {
        id: "b".into(),
        rect: rect(1.0, 1.0),
    });
    ben.act(Command::SetRect {
        id: "b".into(),
        rect: rect(2.0, 2.0),
    });
    exchange(&mut [&mut ann, &mut ben, &mut cat]);
    assert_same(&ann.doc, &ben.doc);
    assert_same(&ann.doc, &cat.doc);
}

#[test]
fn reorders_made_at_the_same_time_converge() {
    let mut ann = Peer::publish(three_stickies());
    let mut ben = Peer::join(&ann);
    let mut cat = Peer::join(&ann);

    let order = |ids: &[&str]| {
        Command::SetOrder(ids.iter().map(|id| ItemId::Entity((*id).into())).collect())
    };
    ann.act(order(&["c", "a", "b"]));
    ben.act(order(&["b", "c", "a"]));
    // Two inserts at the same place.
    cat.act(Command::InsertEntity {
        entity: Box::new(sticky("x", rect(0.0, 0.0), "X")),
        at: 1,
    });
    ann.act(Command::InsertEntity {
        entity: Box::new(sticky("y", rect(0.0, 0.0), "Y")),
        at: 1,
    });
    exchange(&mut [&mut ann, &mut ben, &mut cat]);
    assert_same(&ann.doc, &ben.doc);
    assert_same(&ann.doc, &cat.doc);
    assert_eq!(ann.doc.stack_len(), 5);

    // Keys tied by the concurrent inserts still leave room between them.
    let middle = ann.doc.order()[2].clone();
    let mut wanted: Vec<ItemId> = ann.doc.order().to_vec();
    wanted.retain(|item| *item != middle);
    wanted.insert(1, middle);
    ben.act(Command::SetOrder(wanted.clone()));
    exchange(&mut [&mut ann, &mut ben, &mut cat]);
    assert_eq!(ann.doc.order(), wanted.as_slice());
    assert_same(&ann.doc, &cat.doc);
}

#[test]
fn edits_made_offline_merge_when_peers_reconnect() {
    let mut ann = Peer::publish(three_stickies());
    let mut ben = Peer::join(&ann);

    // Both edit with no connection: what each sends is lost.
    for step in 0..5 {
        let x = f64::from(step) * 10.0;
        ann.act(Command::SetRect {
            id: "a".into(),
            rect: rect(x, 0.0),
        });
        ben.act(Command::SetKind {
            id: "b".into(),
            kind: Box::new(sticky("b", rect(0.0, 0.0), &format!("B{step}")).kind),
        });
    }
    ben.act(Command::InsertEntity {
        entity: Box::new(page("p", rect(0.0, 500.0))),
        at: 0,
    });
    ann.sync.take_outgoing();
    ben.sync.take_outgoing();

    // On reconnect each sends what the other has not seen.
    let for_ben = ann.sync.diff(&ben.sync.state_vector()).expect("diff");
    let for_ann = ben.sync.diff(&ann.sync.state_vector()).expect("diff");
    ben.sync
        .apply_remote(&for_ben, &mut ben.doc)
        .expect("merge");
    ann.sync
        .apply_remote(&for_ann, &mut ann.doc)
        .expect("merge");

    assert_same(&ann.doc, &ben.doc);
    assert_eq!(ann.rect("a"), rect(40.0, 0.0));
    assert!(ann.has("p"));
    let Kind::Text(text) = &ann.doc.entity(&"b".into()).expect("b").kind else {
        panic!("b is a text");
    };
    assert_eq!(text.text, "B4");
}

#[test]
fn undo_takes_back_only_my_own_step() {
    struct Case {
        name: &'static str,
        /// What the peer does after my step and before my undo.
        theirs: Option<Command>,
        rect_after_undo: Rect,
        label_after_undo: Option<&'static str>,
    }
    let start = rect(0.0, 0.0);
    let mine = rect(100.0, 0.0);
    let cases = [
        Case {
            name: "nobody else touched it",
            theirs: None,
            rect_after_undo: start,
            label_after_undo: None,
        },
        Case {
            name: "a peer renamed it",
            theirs: Some(Command::SetLabel {
                id: "a".into(),
                label: Some("theirs".into()),
            }),
            rect_after_undo: start,
            label_after_undo: Some("theirs"),
        },
        Case {
            name: "a peer moved it after me",
            theirs: Some(Command::SetRect {
                id: "a".into(),
                rect: rect(0.0, 500.0),
            }),
            rect_after_undo: rect(0.0, 500.0),
            label_after_undo: None,
        },
    ];
    for case in cases {
        let mut me = Peer::publish(three_stickies());
        let mut peer = Peer::join(&me);
        me.act(Command::SetRect {
            id: "a".into(),
            rect: mine,
        });
        exchange(&mut [&mut me, &mut peer]);
        if let Some(theirs) = case.theirs {
            peer.act(theirs);
            exchange(&mut [&mut me, &mut peer]);
        }

        me.sync.undo(&mut me.doc);
        exchange(&mut [&mut me, &mut peer]);
        assert_eq!(me.rect("a"), case.rect_after_undo, "{}", case.name);
        assert_eq!(
            me.label("a").as_deref(),
            case.label_after_undo,
            "{}",
            case.name
        );
        assert_same(&me.doc, &peer.doc);

        // The peer's own step is still theirs to undo.
        assert!(!peer.sync.can_redo(), "{}", case.name);
    }
}

#[test]
fn undo_and_redo_of_an_insert_and_a_removal_reach_peers() {
    let mut me = Peer::publish(document([
        group("g", rect(0.0, 0.0)),
        inside("g", sticky("a", rect(10.0, 10.0), "A")),
    ]));
    let mut peer = Peer::join(&me);

    me.act(Command::RemoveEntity("a".into()));
    me.act(Command::InsertEntity {
        entity: Box::new(sticky("n", rect(0.0, 0.0), "N")),
        at: 0,
    });
    exchange(&mut [&mut me, &mut peer]);
    let after = me.doc.clone();

    me.sync.undo(&mut me.doc);
    me.sync.undo(&mut me.doc);
    exchange(&mut [&mut me, &mut peer]);
    assert!(peer.has("a") && !peer.has("n"));
    assert_eq!(
        peer.doc.entity(&"a".into()).expect("a").parent,
        Some("g".into())
    );
    assert_eq!(peer.doc.order().first(), Some(&ItemId::Entity("g".into())));
    assert_same(&me.doc, &peer.doc);

    me.sync.redo(&mut me.doc);
    me.sync.redo(&mut me.doc);
    exchange(&mut [&mut me, &mut peer]);
    assert_same(&me.doc, &after);
    assert_same(&peer.doc, &after);
}

#[test]
fn undo_puts_back_the_selection_each_step_carried() {
    let mut me = Peer::publish(three_stickies());
    let before = vec![EntityId::from("a")];
    let after = vec![EntityId::from("b")];
    me.sync
        .apply_from(
            &mut me.doc,
            Command::SetRect {
                id: "a".into(),
                rect: rect(5.0, 5.0),
            },
            before.clone(),
        )
        .expect("applies");
    me.sync.settle(after.clone());
    assert_eq!(me.sync.undo(&mut me.doc), Some(before));
    assert_eq!(me.sync.redo(&mut me.doc), Some(after));
    assert_eq!(me.rect("a"), rect(5.0, 5.0));
}

#[test]
fn a_drag_is_shared_frame_by_frame_and_undone_in_one_step() {
    let mut me = Peer::publish(three_stickies());
    let mut peer = Peer::join(&me);
    let start = me.rect("a");

    // Frames go straight into the document, as `live::write` does, and are
    // shared as they happen.
    for x in [10.0, 20.0, 30.0] {
        let frame = Command::SetRect {
            id: "a".into(),
            rect: rect(x, 0.0),
        };
        me.doc.apply(frame.clone()).expect("frame");
        me.sync.record(&me.doc, &frame);
        exchange(&mut [&mut me, &mut peer]);
        assert_eq!(peer.rect("a"), rect(x, 0.0));
    }

    // On release the drag puts the start back and applies the whole move as
    // one step, as `live::commit_following` does.
    me.doc
        .apply(Command::SetRect {
            id: "a".into(),
            rect: start,
        })
        .expect("restore");
    me.act(Command::SetRect {
        id: "a".into(),
        rect: rect(30.0, 0.0),
    });
    exchange(&mut [&mut me, &mut peer]);
    assert_eq!(peer.rect("a"), rect(30.0, 0.0));

    me.sync.undo(&mut me.doc);
    exchange(&mut [&mut me, &mut peer]);
    assert_eq!(me.rect("a"), start);
    assert_eq!(peer.rect("a"), start);
    assert!(!me.sync.can_undo());
}

#[test]
fn typing_in_one_note_at_the_same_time_keeps_both() {
    let mut start = three_stickies();
    start
        .apply(Command::SetNote {
            file: "plan.md".into(),
            text: Some("🚀 one\ntwo\n".into()),
        })
        .expect("note");
    let mut ann = Peer::publish(start);
    let mut ben = Peer::join(&ann);

    ann.act(Command::SetNote {
        file: "plan.md".into(),
        text: Some("🚀 one!\ntwo\n".into()),
    });
    ben.act(Command::SetNote {
        file: "plan.md".into(),
        text: Some("🚀 one\ntwo\nthree ✅\n".into()),
    });
    exchange(&mut [&mut ann, &mut ben]);
    assert_eq!(ann.doc.note("plan.md"), Some("🚀 one!\ntwo\nthree ✅\n"));
    assert_same(&ann.doc, &ben.doc);

    // My undo takes out my typing and leaves theirs.
    ann.sync.undo(&mut ann.doc);
    exchange(&mut [&mut ann, &mut ben]);
    assert_eq!(ben.doc.note("plan.md"), Some("🚀 one\ntwo\nthree ✅\n"));
}
