//! The prompts a run starts with.

use specular_agent::{
    CommentContext, Image, MediaType, Message, Pill, PromptContext, Role, Status, Thread, ThreadId,
    WriteTarget, follow_up_prompt, thread_prompt,
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
            message(Role::Agent, "Done.", None),
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

#[test]
fn the_repo_target_works_in_the_repo_and_reads_images_from_the_space() {
    let target = WriteTarget::Repo {
        origin: "http://localhost:3000".into(),
        repo_path: "/repo".into(),
    };
    let mut thread = thread();
    thread.messages[2].images.push(Image {
        path: ".specular/threads/tab/attachments/t1/img_1.png".into(),
        media_type: MediaType::Png,
    });
    let prompt = thread_prompt(&thread, &context(target.clone(), Pill::Empty));
    let head: Vec<&str> = prompt.lines().take(9).collect();
    assert_eq!(
        head,
        [
            "Working directory (linked repo): /repo",
            "Space folder: /space",
            "This turn should change source for http://localhost:3000 in the repo at /repo.",
            "Edit that repo. Pages already on the canvas reload from source.",
            "Anything new reaches the user only once it is on the canvas: `specular add page <full url> --at x,y`.",
            "",
            "Current selection: Home",
            "",
            "Images the user pasted this turn are attached; earlier ones are at the listed paths.",
        ]
    );
    assert!(prompt.contains(
        "[User] And bigger [image: /space/.specular/threads/tab/attachments/t1/img_1.png]"
    ));

    let space = thread_prompt(&thread, &context(WriteTarget::Space, Pill::Empty));
    assert!(space.contains("[image: .specular/threads/tab/attachments/t1/img_1.png]"));
}

#[test]
fn the_follow_up_is_short_and_names_the_comments_in_the_turn() {
    let pill = Pill::Annotation {
        label: "button.cta".into(),
        annotation_id: "c1".into(),
    };
    let ctx = context(WriteTarget::Space, pill);
    let prompt = follow_up_prompt("  Make it green ", &ctx, 2, &["c1".into(), "gone".into()]);
    let expected = format!(
        "The user followed up in the same canvas agent thread:
[User] Make it green
  (comment c1 on element \"button.cta\" of page p1 (http://localhost:3000/))
(2 images are attached to this message.)

The user has selected comment c1 and likely wants to focus on that.
Prefer changing those. Touch other canvas items only if the request clearly needs it.

Continue the thread — make a change if it calls for one, or just answer if it is a question.
Use the specular skill as before.

{REPLY}"
    );
    assert_eq!(prompt, expected);

    let bare = follow_up_prompt("", &context(WriteTarget::Space, Pill::Empty), 1, &[]);
    assert!(
        bare.contains("[User] See the attached image.\n(An image is attached to this message.)\n")
    );
    let none = follow_up_prompt(" ", &context(WriteTarget::Space, Pill::Empty), 0, &[]);
    assert!(none.contains("[User] Continue addressing the latest feedback in this thread.\n\nCurrent selection: Home\n"));
}
