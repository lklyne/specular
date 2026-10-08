//! Sending a thread, the run's life, and what each notice does to the store.

mod common;

use common::{TAB, comment, ctx, id, send};
use specular_agent::{
    Notice, Outcome, Progress, ProgressKind, RunState, Status, ThreadId, Threads,
};

const NOW: &str = "2026-01-01T00:00:09Z";

fn finish(
    threads: &mut Threads,
    thread: &ThreadId,
    session: Option<&str>,
    text: &str,
    message: &str,
) -> Outcome {
    if let Some(s) = session {
        threads.on_notice(thread, Notice::Session(s.into()), message, NOW);
    }
    threads.on_notice(thread, Notice::Finished { text: text.into() }, message, NOW)
}

#[expect(clippy::unwrap_used, reason = "a test fails by panicking")]
/// A thread that has been answered once under session `s1`, with one more message typed.
fn answered(threads: &mut Threads) -> ThreadId {
    let thread = comment(threads, "t1", "m1", "pin1", "make it blue");
    threads.begin_run(&thread, NOW, &ctx()).unwrap();
    finish(threads, &thread, Some("s1"), "Done.\n<<RESOLVE>>", "a1");
    thread
}

#[test]
fn send_starts_a_run_with_the_full_prompt_and_clears_the_queue() {
    let mut threads = Threads::default();
    let thread = comment(&mut threads, "t1", "m1", "pin1", "make it blue");
    let started = threads
        .begin_run(&thread, "2026-02-02T00:00:00Z", &ctx())
        .unwrap();
    assert_eq!(started.request.thread, thread);
    assert_eq!(started.request.resume, None);
    assert!(
        started
            .request
            .prompt
            .contains("Thread:\n[User] make it blue\n")
    );
    assert_eq!(started.changed.threads, std::slice::from_ref(&thread));
    assert_eq!(threads.queued(&thread).len(), 0);
    assert_eq!(
        threads.get(&thread).unwrap().updated_at,
        "2026-02-02T00:00:00Z"
    );
    assert!(threads.is_running(&thread));
    assert_eq!(threads.run(&thread).unwrap().current_label(), "Starting");
    assert!(
        threads.begin_run(&thread, NOW, &ctx()).is_none(),
        "one run at a time"
    );
}

#[test]
fn a_thread_with_no_user_words_does_not_run() {
    let mut threads = Threads::default();
    threads.new_thread(TAB, id("t1"), NOW);
    assert!(threads.begin_run(&id("t1"), NOW, &ctx()).is_none());
    assert!(threads.begin_run(&id("nope"), NOW, &ctx()).is_none());
    assert!(!threads.is_running(&id("t1")));
}

#[test]
fn a_run_answers_and_the_next_comment_starts_another_draft() {
    let mut threads = Threads::default();
    let thread = comment(&mut threads, "t1", "m1", "pin1", "make it blue");
    threads.begin_run(&thread, NOW, &ctx()).unwrap();
    let outcome = finish(
        &mut threads,
        &thread,
        Some("s1"),
        "Made it blue.\n<<RESOLVE>>",
        "a1",
    );
    let Outcome::Finished {
        has_queued_turn,
        changed,
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    assert!(!has_queued_turn);
    assert_eq!(changed.threads, std::slice::from_ref(&thread));

    let t = threads.get(&thread).unwrap();
    assert_eq!(
        (t.status, t.claude_session_id.as_deref()),
        (Status::Open, Some("s1"))
    );
    assert_eq!(
        t.messages.last().map(|m| m.text.as_str()),
        Some("Made it blue.")
    );
    assert!(threads.run(&thread).is_none());

    let next = comment(&mut threads, "t2", "m2", "pin2", "another");
    assert_eq!(next, id("t2"));
    assert_eq!(threads.for_canvas(TAB).len(), 2);
}

#[test]
fn a_follow_up_on_an_open_thread_resumes_with_the_follow_up_prompt() {
    let mut threads = Threads::default();
    let thread = answered(&mut threads);
    send(&mut threads, "and bigger", "tx", "m3");
    let started = threads.begin_run(&thread, NOW, &ctx()).unwrap();
    assert_eq!(started.request.resume.as_deref(), Some("s1"));
    assert!(
        started.request.prompt.starts_with(
            "The user followed up in the same canvas agent thread:\n[User] and bigger\n"
        )
    );

    // Nothing queued: resume still, but with the whole thread.
    finish(&mut threads, &thread, None, "ok", "a2");
    let again = threads.begin_run(&thread, NOW, &ctx()).unwrap();
    assert_eq!(
        again.request.resume.as_deref(),
        Some("s1"),
        "no new session reported keeps the old one"
    );
    assert!(again.request.prompt.starts_with("Working directory"));
}

#[test]
fn a_message_sent_during_a_run_stays_queued_and_finished_says_to_drain() {
    let mut threads = Threads::default();
    let thread = answered(&mut threads);
    send(&mut threads, "first follow-up", "tx", "m3");
    threads.begin_run(&thread, NOW, &ctx()).unwrap();

    send(&mut threads, "typed while running", "tx", "m4");
    assert!(threads.begin_run(&thread, NOW, &ctx()).is_none());
    assert_eq!(threads.queued(&thread).len(), 1);

    let Outcome::Finished {
        has_queued_turn, ..
    } = finish(&mut threads, &thread, Some("s1"), "ok", "a2")
    else {
        panic!("not finished")
    };
    assert!(has_queued_turn);
    let started = threads.begin_run(&thread, NOW, &ctx()).unwrap();
    assert!(
        started
            .request
            .prompt
            .contains("[User] typed while running")
    );
}

#[test]
fn the_newest_queued_comment_is_where_a_drained_turn_aims() {
    let mut threads = Threads::default();
    let thread = comment(&mut threads, "t1", "m1", "pin1", "one");
    comment(&mut threads, "t2", "m2", "pin2", "two");
    send(&mut threads, "plain", "t3", "m3");
    assert_eq!(threads.last_queued_annotation(&thread), Some("pin2"));
    threads.begin_run(&thread, NOW, &ctx()).unwrap();
    assert_eq!(threads.last_queued_annotation(&thread), None);
}

#[test]
fn a_failed_resume_retries_fresh_with_the_full_prompt() {
    let mut threads = Threads::default();
    let thread = answered(&mut threads);
    send(&mut threads, "and bigger", "tx", "m3");
    threads.begin_run(&thread, NOW, &ctx()).unwrap();

    let outcome = threads.on_notice(
        &thread,
        Notice::Failed {
            error: "no such session".into(),
        },
        "a2",
        NOW,
    );
    let Outcome::Retry { request, changed } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(request.resume, None);
    assert!(request.prompt.starts_with("Working directory"));
    assert!(request.prompt.contains("[User] and bigger"));
    assert_eq!(changed.threads, std::slice::from_ref(&thread));
    assert_eq!(threads.get(&thread).unwrap().claude_session_id, None);
    let run = threads.run(&thread).unwrap();
    assert_eq!(run.state, RunState::Running);
    assert_eq!(
        run.events.last().map(|e| e.text.as_str()),
        Some("Could not resume prior session — starting fresh.")
    );

    let outcome = threads.on_notice(
        &thread,
        Notice::Failed {
            error: "still no".into(),
        },
        "a2",
        NOW,
    );
    assert_eq!(outcome, Outcome::Failed, "only one retry");
}

#[test]
fn a_failed_first_run_keeps_the_queue_and_shows_the_error() {
    let mut threads = Threads::default();
    let thread = comment(&mut threads, "t1", "m1", "pin1", "go");
    threads.begin_run(&thread, NOW, &ctx()).unwrap();
    send(&mut threads, "later", "tx", "m2");
    let outcome = threads.on_notice(
        &thread,
        Notice::Failed {
            error: "Not logged in".into(),
        },
        "a1",
        NOW,
    );
    assert_eq!(outcome, Outcome::Failed);
    assert_eq!(
        threads.run(&thread).unwrap().state,
        RunState::Failed("Not logged in".into())
    );
    assert!(!threads.is_running(&thread));
    assert_eq!(threads.queued(&thread).len(), 1);
    assert_eq!(threads.get(&thread).unwrap().status, Status::Draft);

    let again = threads.begin_run(&thread, NOW, &ctx()).unwrap();
    assert!(again.request.prompt.contains("[User] later"));
    assert_eq!(threads.run(&thread).unwrap().state, RunState::Running);
}

#[test]
fn cancel_removes_the_run_and_appends_nothing() {
    let mut threads = Threads::default();
    let thread = comment(&mut threads, "t1", "m1", "pin1", "go");
    threads.begin_run(&thread, NOW, &ctx()).unwrap();
    assert_eq!(
        threads.on_notice(&thread, Notice::Cancelled, "a1", NOW),
        Outcome::Cancelled
    );
    assert!(threads.run(&thread).is_none());
    assert_eq!(threads.get(&thread).unwrap().messages.len(), 1);
    assert_eq!(
        threads.on_notice(&thread, Notice::Cancelled, "a1", NOW),
        Outcome::Ignored
    );
    assert_eq!(
        threads.on_notice(&thread, Notice::Text("late".into()), "a1", NOW),
        Outcome::Ignored
    );
}

#[test]
fn the_run_collects_progress_text_and_a_capped_log() {
    let mut threads = Threads::default();
    let thread = comment(&mut threads, "t1", "m1", "pin1", "go");
    threads.begin_run(&thread, NOW, &ctx()).unwrap();
    let line = |text: String, label: Option<&str>| Progress {
        kind: ProgressKind::Text,
        text,
        label: label.map(Into::into),
    };
    threads.on_notice(&thread, Notice::Text("Hel".into()), "a", NOW);
    threads.on_notice(&thread, Notice::TextDelta("lo".into()), "a", NOW);
    threads.on_notice(
        &thread,
        Notice::Progress(line("one".into(), Some("Reading a.ts"))),
        "a",
        NOW,
    );
    threads.on_notice(
        &thread,
        Notice::Progress(line("result".into(), None)),
        "a",
        NOW,
    );
    let run = threads.run(&thread).unwrap();
    assert_eq!(run.text, "Hello");
    assert_eq!(
        run.current_label(),
        "Reading a.ts",
        "a result carries no label, so the step in flight stays"
    );

    for n in 0..250 {
        threads.on_notice(
            &thread,
            Notice::Progress(line(format!("e{n}"), None)),
            "a",
            NOW,
        );
    }
    let run = threads.run(&thread).unwrap();
    assert_eq!(
        (run.events.len(), run.events[0].text.as_str()),
        (200, "e50")
    );
    assert_eq!(
        run.current_label(),
        "Starting",
        "the labelled line scrolled out"
    );
}
