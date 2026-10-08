//! A thread file is written and read as the Electron app does.

use std::collections::BTreeMap;

use specular_agent::{
    Image, MediaType, Message, Role, Status, Thread, ThreadId, index_json, parse_index,
    title_from_messages,
};

const NOW: &str = "2026-01-01T00:00:00.000Z";

fn user(id: &str, text: &str) -> Message {
    Message {
        id: id.into(),
        role: Role::User,
        text: text.into(),
        created_at: "2026-01-02T00:00:00.000Z".into(),
        queued: false,
        annotation_id: None,
        images: Vec::new(),
    }
}

#[test]
fn a_thread_is_written_in_the_electron_shape() {
    let thread = Thread {
        id: ThreadId("thread_1".into()),
        tab_id: "tab_a".into(),
        title: "Fix the button".into(),
        status: Status::Open,
        created_at: "2026-01-02T00:00:00.000Z".into(),
        updated_at: "2026-01-03T00:00:00.000Z".into(),
        claude_session_id: Some("sess-1".into()),
        annotation_ids: vec!["a1".into()],
        messages: vec![
            Message {
                queued: true,
                annotation_id: Some("a1".into()),
                ..user("m1", "Fix the button")
            },
            Message {
                images: vec![Image {
                    path: "img/1.png".into(),
                    media_type: MediaType::Png,
                }],
                ..user("m2", "see")
            },
            Message {
                role: Role::Agent,
                ..user("m3", "Done.")
            },
        ],
    };
    let expected = r#"{
  "id": "thread_1",
  "tabId": "tab_a",
  "title": "Fix the button",
  "status": "open",
  "createdAt": "2026-01-02T00:00:00.000Z",
  "updatedAt": "2026-01-03T00:00:00.000Z",
  "claudeSessionId": "sess-1",
  "annotationIds": [
    "a1"
  ],
  "messages": [
    {
      "id": "m1",
      "role": "user",
      "text": "Fix the button",
      "createdAt": "2026-01-02T00:00:00.000Z",
      "queued": true,
      "annotationId": "a1"
    },
    {
      "id": "m2",
      "role": "user",
      "text": "see",
      "createdAt": "2026-01-02T00:00:00.000Z",
      "images": [
        {
          "path": "img/1.png",
          "mediaType": "image/png"
        }
      ]
    },
    {
      "id": "m3",
      "role": "agent",
      "text": "Done.",
      "createdAt": "2026-01-02T00:00:00.000Z"
    }
  ]
}
"#;
    assert_eq!(thread.to_json(), expected);
    assert_eq!(Thread::from_json(expected, "other", NOW), Some(thread));
}

#[test]
fn an_electron_file_reads_tolerantly() {
    let file = r#"{
  "id": "t9",
  "title": "   ",
  "status": "closed",
  "createdAt": "2026-02-01T00:00:00.000Z",
  "annotationIds": ["a1", 4, "a2"],
  "messages": [
    {"id": "m1", "role": "user", "text": "hi", "queued": true, "annotationId": "a1",
     "images": [{"path": "p.png", "mediaType": "image/png"}, {"path": "q.bmp", "mediaType": "image/bmp"}, 7]},
    {"id": "m2", "role": "robot", "text": "bad role"},
    {"id": "m3", "role": "agent"},
    "not a message",
    {"id": "m4", "role": "agent", "text": "ok", "queued": "yes"}
  ]
}"#;
    let thread = Thread::from_json(file, "tab_fallback", NOW).expect("parses");
    assert_eq!(thread.tab_id, "tab_fallback");
    assert_eq!(thread.title, "New thread");
    assert_eq!(thread.status, Status::Closed);
    assert_eq!(thread.updated_at, "2026-02-01T00:00:00.000Z");
    assert_eq!(thread.claude_session_id, None);
    assert_eq!(thread.annotation_ids, ["a1", "a2"]);
    let ids: Vec<&str> = thread.messages.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["m1", "m4"]);
    assert!(thread.messages[0].queued && !thread.messages[1].queued);
    assert_eq!(thread.messages[0].created_at, NOW);
    assert_eq!(thread.messages[0].images.len(), 1);
    assert_eq!(thread.messages[0].annotation_id.as_deref(), Some("a1"));
}

#[test]
fn files_that_are_not_threads_are_refused() {
    for text in [
        "",
        "[]",
        r#"{"id":"t","status":"archived"}"#,
        r#"{"id":"t"}"#,
        r#"{"id":"","status":"open"}"#,
        r#"{"status":"open"}"#,
    ] {
        assert_eq!(Thread::from_json(text, "tab", NOW), None, "{text}");
    }
}

#[test]
fn the_index_round_trips_and_garbage_is_empty() {
    let mut by_canvas = BTreeMap::new();
    by_canvas.insert("tab_a".to_owned(), ThreadId("t1".into()));
    by_canvas.insert("tab_b".to_owned(), ThreadId("t2".into()));
    let text = index_json(Some(&ThreadId("t1".into())), &by_canvas);
    assert_eq!(
        text,
        "{\n  \"activeThreadId\": \"t1\",\n  \"activeByCanvas\": {\n    \"tab_a\": \"t1\",\n    \"tab_b\": \"t2\"\n  }\n}\n"
    );
    let index = parse_index(&text);
    assert_eq!(index.active, Some(ThreadId("t1".into())));
    assert_eq!(index.by_canvas, by_canvas);

    assert!(index_json(None, &BTreeMap::new()).contains("\"activeThreadId\": null"));
    let electron = parse_index(r#"{"activeThreadId":"t3"}"#);
    assert_eq!(
        (electron.active, electron.by_canvas.len()),
        (Some(ThreadId("t3".into())), 0)
    );
    for garbage in [
        "",
        "nope",
        "[1]",
        r#"{"activeThreadId":5,"activeByCanvas":[]}"#,
    ] {
        assert_eq!(parse_index(garbage), parse_index("{}"), "{garbage}");
    }
}

#[test]
fn a_title_is_the_first_user_text_cut_to_48_characters() {
    let long = "é".repeat(60);
    let image = Message {
        images: vec![Image {
            path: "a.png".into(),
            media_type: MediaType::Png,
        }],
        ..user("m", "  ")
    };
    let cases: [(Vec<Message>, String); 5] = [
        (vec![], "New thread".into()),
        (vec![user("m", "  a   b\n c ")], "a b c".into()),
        (vec![user("m", &long)], format!("{}…", "é".repeat(47))),
        (vec![image.clone()], "Image".into()),
        (vec![image, user("n", "words")], "words".into()),
    ];
    for (messages, title) in cases {
        assert_eq!(title_from_messages(&messages), title);
    }
}
