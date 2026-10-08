//! Which thread a comment or message lands in, and the per-canvas active thread.

mod common;

use common::{TAB, comment, ctx, id, send};
use specular_agent::{Changed, Index, Status, Threads};

const NOW: &str = "2026-01-01T00:00:09Z";

#[test]
fn a_comment_queues_into_the_draft_and_a_second_joins_it() {
    let mut threads = Threads::default();
    let (first, changed) =
        threads.queue_comment(TAB, id("t1"), "m1", "pin1", " make it blue ", NOW);
    assert_eq!(
        (first.clone(), changed),
        (
            id("t1"),
            Changed {
                threads: vec![id("t1")],
                index: true
            }
        )
    );
    let (second, changed) = threads.queue_comment(TAB, id("t2"), "m2", "pin2", "", NOW);
    assert_eq!(
        (second, changed),
        (
            id("t1"),
            Changed {
                threads: vec![id("t1")],
                index: false
            }
        )
    );

    let thread = threads.get(&first).unwrap();
    assert_eq!(
        (thread.status, thread.title.as_str()),
        (Status::Draft, "make it blue")
    );
    assert_eq!(thread.annotation_ids, ["pin1", "pin2"]);
    let texts: Vec<&str> = thread.messages.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, ["make it blue", "(comment)"]);
    assert!(thread.messages.iter().all(|m| m.queued));
    assert_eq!(threads.queued(&first).len(), 2);
    assert_eq!(threads.active(TAB).unwrap().id, first);
    assert_eq!(threads.all().len(), 1);
}

#[test]
fn a_pin_maps_to_one_thread() {
    let mut threads = Threads::default();
    let first = comment(&mut threads, "t1", "m1", "pin1", "one");
    threads.begin_run(&first, NOW, &ctx()).unwrap();
    let _ = threads.on_notice(
        &first,
        specular_agent::Notice::Finished { text: "ok".into() },
        "a1",
        NOW,
    );
    let second = comment(&mut threads, "t2", "m2", "pin2", "two");
    assert_ne!(
        first, second,
        "an answered thread is not a draft, so the next comment starts one"
    );
    assert_eq!(threads.active(TAB).unwrap().id, second);

    let (back, changed) = threads.queue_comment(TAB, id("t3"), "m3", "pin1", "again", NOW);
    assert_eq!(back, first);
    assert!(changed.index, "the pin's thread becomes active");
    assert_eq!(threads.active(TAB).unwrap().id, first);
    assert_eq!(threads.get(&first).unwrap().annotation_ids, ["pin1"]);
    assert_eq!(threads.all().len(), 2);
}

#[test]
fn the_composer_text_queues_on_the_active_thread_or_starts_one() {
    let mut threads = Threads::default();
    assert!(
        threads
            .queue_message(TAB, id("t1"), "m1", "  ", Vec::new(), NOW)
            .is_none()
    );
    let first = send(&mut threads, " hello ", "t1", "m1");
    assert_eq!(first, id("t1"));
    assert_eq!(
        send(&mut threads, "more", "t2", "m2"),
        first,
        "the active thread takes it"
    );
    assert_eq!(threads.queued(&first).len(), 2);

    let (same, changed) = threads
        .queue_message(TAB, id("t3"), "m3", "", Vec::new(), NOW)
        .unwrap();
    assert_eq!((same, changed), (first.clone(), Changed::default()));
    assert_eq!(threads.get(&first).unwrap().messages.len(), 2);

    threads.close(&first, NOW);
    assert_eq!(
        send(&mut threads, "fresh", "t4", "m4"),
        id("t4"),
        "a closed thread is not active"
    );
}

#[test]
fn new_select_and_deselect_keep_one_active_thread_per_canvas() {
    let mut threads = Threads::default();
    let changed = threads.new_thread(TAB, id("t1"), NOW);
    assert_eq!(
        changed,
        Changed {
            threads: vec![id("t1")],
            index: true
        }
    );
    let t = threads.get(&id("t1")).unwrap();
    assert_eq!(
        (t.title.as_str(), t.status, t.messages.len()),
        ("New thread", Status::Draft, 0)
    );
    threads.new_thread("tab_b", id("t2"), NOW);
    assert_eq!(threads.active(TAB).unwrap().id, id("t1"));
    assert_eq!(threads.active("tab_b").unwrap().id, id("t2"));

    assert!(
        !threads.select(TAB, &id("t2")).index,
        "another canvas's thread is refused"
    );
    assert!(threads.deselect(TAB).index);
    assert!(threads.active(TAB).is_none());
    assert!(!threads.deselect(TAB).index);
    assert!(threads.select(TAB, &id("t1")).index);
    assert_eq!(threads.active(TAB).unwrap().id, id("t1"));
    assert_eq!(threads.active("tab_b").unwrap().id, id("t2"));
}

#[test]
fn closing_archives_a_thread_out_of_the_list_but_not_the_store() {
    let mut threads = Threads::default();
    threads.new_thread(TAB, id("t1"), "2026-01-01T00:00:01Z");
    threads.new_thread(TAB, id("t2"), "2026-01-01T00:00:02Z");
    let names = |t: &Threads| {
        t.for_canvas(TAB)
            .iter()
            .map(|t| t.id.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&threads), ["t2", "t1"], "newest first");

    let changed = threads.close(&id("t2"), NOW);
    assert_eq!(
        changed,
        Changed {
            threads: vec![id("t2")],
            index: true
        }
    );
    assert_eq!(names(&threads), ["t1"]);
    assert_eq!(threads.get(&id("t2")).unwrap().status, Status::Closed);
    assert!(threads.active(TAB).is_none());
    assert!(
        threads.select(TAB, &id("t2")) == Changed::default(),
        "a closed thread cannot be opened"
    );
    assert_eq!(threads.all().len(), 2);
}

#[test]
fn closing_is_refused_mid_run() {
    let mut threads = Threads::default();
    let t = comment(&mut threads, "t1", "m1", "pin1", "go");
    threads.begin_run(&t, NOW, &ctx()).unwrap();
    assert_eq!(threads.close(&t, NOW), Changed::default());
    assert_eq!(threads.get(&t).unwrap().status, Status::Draft);
    assert!(threads.active(TAB).is_some());
}

#[test]
fn load_drops_an_active_id_that_names_nothing_or_a_closed_thread() {
    let mut source = Threads::default();
    source.new_thread(TAB, id("t1"), NOW);
    source.new_thread("tab_b", id("t2"), NOW);
    source.new_thread("tab_c", id("t3"), NOW);
    source.close(&id("t3"), NOW);
    let files = source.all().to_vec();

    let mut index = Index::default();
    index.by_canvas.insert(TAB.into(), id("t1"));
    index.by_canvas.insert("tab_b".into(), id("ghost"));
    index.by_canvas.insert("tab_c".into(), id("t3"));
    let mut threads = Threads::default();
    threads.load(files.clone(), &index);
    assert_eq!(threads.active(TAB).unwrap().id, id("t1"));
    assert!(threads.active("tab_b").is_none());
    assert!(threads.active("tab_c").is_none());
    assert_eq!(threads.all().len(), 3);

    // Electron's single id still finds its canvas.
    let electron = Index {
        active: Some(id("t2")),
        ..Index::default()
    };
    threads.load(files, &electron);
    assert_eq!(threads.active("tab_b").unwrap().id, id("t2"));
    assert!(threads.active(TAB).is_none());
    assert!(
        threads
            .index_json(Some("tab_b"))
            .contains("\"activeThreadId\": \"t2\"")
    );
}
