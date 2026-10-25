//! The prompts a run starts with.

use specular_agent::{
    CommentContext, Message, Pill, PromptContext, Role, Status, Thread, ThreadId, WriteTarget,
    thread_prompt,
};

fn message(role: Role, text: &str, annotation: Option<&str>) -> Message {
    Message {
        id: "m".into(),
        role,
        text: text.into(),
        created_at: "t".into(),
        queued: false,
        annotation_id: annotation.map(Into::into),
        images: Vec::new(),
        artifacts: Vec::new(),
    }
}

fn thread() -> Thread {
    Thread {
        id: ThreadId("t1".into()),
        tab_id: "tab".into(),
        title: "x".into(),
        status: Status::Open,
        created_at: "t".into(),
        updated_at: "t".into(),
        claude_session_id: None,
        annotation_ids: vec!["c1".into()],
        messages: vec![
            message(Role::User, "Make it blue", Some("c1")),
            message(Role::Agent, "Done.", Some("c1")),
            message(Role::User, "And bigger", None),
        ],
    }
}

fn context(target: WriteTarget, pill: Pill) -> PromptContext {
    PromptContext {
        space_path: "/space".into(),
        write_target: target,
        pill,
        canvas_name: "Home".into(),
        comments: vec![CommentContext {
            annotation_id: "c1".into(),
            description: "on element \"button.cta\" of page p1 (http://localhost:3000/)".into(),
        }],
    }
}

const REPLY: &str = "Reply format — REQUIRED:
- Your entire final message is the only thing the user sees, so keep it brief and self-contained — no references to your steps, tool output, or anything \"above\" they cannot see.
- End the message with one of:
  <<RESOLVE>>   if you have addressed the request (made the change or answered it)
  <<WAITING>>   if you need more information from the user
Do not write anything after the marker.";

#[test]
fn the_full_prompt_carries_the_thread_and_what_each_comment_points_at() {
    let selection = Pill::Selection {
        label: "2 items".into(),
        entity_ids: vec!["a".into(), "b".into()],
    };
    let prompt = thread_prompt(&thread(), &context(WriteTarget::Space, selection));
    let expected = format!(
        "Working directory (space folder): /space
This turn is about the canvas / space, not a linked site repo.
Use `specular add` / `update` / `delete` / `arrange` to change what the user sees.
Files you write in this folder reach the user once they are on the canvas: `specular add file <path> --at x,y`.

The user has selected a, b and likely wants to focus on those.
Prefer changing those. Touch other canvas items only if the request clearly needs it.

Thread:
[User] Make it blue
  (comment c1 on element \"button.cta\" of page p1 (http://localhost:3000/))
[Agent] Done.
[User] And bigger

Inspecting a live page (when one is in play):
  specular snapshot -i -f <pageId>
  specular get styles @<ref>
  specular screenshot -f <pageId>

{REPLY}"
    );
    assert_eq!(prompt, expected);
}
