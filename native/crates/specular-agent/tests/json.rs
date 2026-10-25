//! A thread file is written and read as the Electron app does.

use specular_agent::{
    Artifact, ArtifactTarget, Image, MediaType, Message, Role, Status, Thread, ThreadId,
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
        artifacts: Vec::new(),
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
                artifacts: vec![
                    Artifact {
                        target: ArtifactTarget::File("/space/plan.md".into()),
                        created: true,
                    },
                    Artifact {
                        target: ArtifactTarget::Item("e1".into()),
                        created: false,
                    },
                ],
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
      "createdAt": "2026-01-02T00:00:00.000Z",
      "artifacts": [
        {
          "file": "/space/plan.md",
          "created": true
        },
        {
          "item": "e1"
        }
      ]
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
