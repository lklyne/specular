//! The agent's side of the chat: what Send runs, what a run reports, and how
//! threads are kept.

use specular_core::{CssSize, synthetic_element_at};
use specular_doc::{Command, Document, Entity, ItemId, Rect};
use specular_interact::{
    Action, ApiCall, ApiRun, Artifact, ArtifactTarget, AssetBytes, ChatAction, Effect, Event,
    ImageUpload, Key, MediaType, Model, ModelsNews, Notice, Progress, ProgressKind, RunRequest,
    ThreadId, ThreadStatus, Tool,
};
use specular_testkit::{TestApp, assert_chat_snapshot, document, note, pages, sticky};

fn panel_app(entities: impl IntoIterator<Item = Entity>) -> TestApp {
    let mut app = TestApp::with_space([("Home", Document::new())]);
    app.open(document(entities)).with_chat_panel();
    app.tool(Tool::Comment).take_effects();
    app
}

/// An app whose open thread has one queued comment.
fn queued() -> TestApp {
    let mut app = panel_app([]);
    app.click((600.0, 500.0)).send_chat("make it blue");
    app.take_effects();
    app
}

fn run_requests(effects: &[Effect]) -> Vec<RunRequest> {
    (effects.iter())
        .filter_map(|effect| match effect {
            Effect::RunAgent(request) => Some((**request).clone()),
            _ => None,
        })
        .collect()
}

fn finished(text: &str) -> Notice {
    Notice::Finished { text: text.into() }
}

fn step(label: &str) -> Notice {
    Notice::Progress(Progress {
        kind: ProgressKind::ToolUse,
        text: format!("{label} the page"),
        label: Some(label.into()),
    })
}

#[test]
fn send_runs_the_agent_with_the_comments_anchor_and_the_space_folder() {
    let mut app = panel_app(pages(1));
    app.click((200.0, 200.0))
        .answer_element(synthetic_element_at(
            CssSize::new(400, 300),
            (100.0, 100.0).into(),
        ));
    app.send_chat("too small");
    app.click((900.0, 700.0)).send_chat("and here");
    app.take_effects();
    app.send_chat("");
    let effects = app.take_effects();
    let thread = app.chat_thread_id().expect("an open thread");
    assert_eq!(effects[0], Effect::WriteThread(thread.clone()));
    let [request] = run_requests(&effects).try_into().expect("one run");
    assert_eq!(request.thread, thread);
    assert_eq!(request.resume, None);
    for line in [
        "Working directory (space folder): /space",
        "[User] too small",
        r#"on element "div.cell[data-col="0"][data-row="2"]" of page p1 (https://example.com/p1)"#,
        "at canvas point (900, 700)",
    ] {
        assert!(
            request.prompt.contains(line),
            "{line:?} is missing from\n{}",
            request.prompt
        );
    }
    let transcript = app.chat().transcript.expect("the thread is open");
    let said: Vec<_> = transcript
        .messages
        .iter()
        .map(|b| b.text.as_str())
        .collect();
    assert_eq!(
        said,
        ["too small", "and here"],
        "the user's words are in the transcript"
    );
    assert_eq!(app.chat().composer.queued, []);
}

#[test]
fn a_run_streams_into_the_model_and_finishing_answers_in_the_thread() {
    let mut app = queued();
    app.send_chat("");
    let thread = app.chat_thread_id().expect("an open thread");
    app.take_effects();
    app.agent_says(Notice::Session("sess-1".into()))
        .agent_says(step("Reading"))
        .agent_says(Notice::Text("Hel".into()))
        .agent_says(Notice::TextDelta("lo".into()));
    assert_chat_snapshot!(app, @r#"
    chat available=yes visible=yes width=400 title="make it blue"
    thread [x] "make it blue" draft 1970-01-01T00:00:00.000Z
    controls back=yes close=yes
    bubble user "make it blue" pin=e220a8397b1dcdaf focused live
    streaming "Hello"
    run "Reading"
      log "Reading the page"
    open-comments 1
    composer placeholder="Queue a follow-up…" send-empty=no running=yes pill=Comment "make it blue" folder=Some("space")
    "#);
    app.agent_says(finished("Made it blue.\n<<RESOLVE>>"));
    let stored = app.app().threads().get(&thread).expect("kept");
    assert_eq!(stored.status, ThreadStatus::Open);
    assert_eq!(stored.claude_session_id.as_deref(), Some("sess-1"));
    assert_eq!(app.take_effects(), [Effect::WriteThread(thread)]);
    assert_chat_snapshot!(app, @r#"
    chat available=yes visible=yes width=400 title="make it blue"
    thread [x] "make it blue" 1970-01-01T00:00:00.000Z
    controls back=yes close=yes
    bubble user "make it blue" pin=e220a8397b1dcdaf focused live
    bubble agent "Made it blue."
    open-comments 1
    composer placeholder="Follow up…" send-empty=no running=no pill=Comment "make it blue" folder=Some("space")
    "#);
}

#[test]
fn a_message_sent_during_a_run_waits_and_goes_as_a_resume_when_the_run_ends() {
    let mut app = queued();
    app.send_chat("")
        .agent_says(Notice::Session("sess-1".into()));
    app.agent_says(finished("ok"));
    app.send_chat("first follow-up");
    app.take_effects();
    app.send_chat("now darker");
    assert!(
        run_requests(&app.take_effects()).is_empty(),
        "a run is in flight"
    );
    let chat = app.chat();
    assert_eq!(chat.composer.queued.len(), 1);
    assert_eq!(chat.composer.placeholder, "Queue a follow-up…");
    app.agent_says(finished("darker now"));
    let [request] = run_requests(&app.take_effects())
        .try_into()
        .expect("the drain");
    assert_eq!(request.resume.as_deref(), Some("sess-1"));
    assert!(
        request.prompt.starts_with("The user followed up"),
        "{}",
        request.prompt
    );
    assert!(request.prompt.contains("[User] now darker"));
}

#[test]
fn a_failed_resume_starts_over_and_a_failed_first_run_shows_its_error() {
    let mut app = queued();
    app.send_chat("")
        .agent_says(Notice::Session("sess-1".into()));
    app.agent_says(finished("ok"));
    app.send_chat("again");
    app.take_effects();
    app.agent_says(Notice::Failed {
        error: "no such session".into(),
    });
    let [retry] = run_requests(&app.take_effects())
        .try_into()
        .expect("one retry");
    assert_eq!(retry.resume, None);
    assert!(
        retry.prompt.contains("Thread:"),
        "the whole thread goes again"
    );
    let thread = app.chat_thread_id().expect("open");
    assert_eq!(
        app.app()
            .threads()
            .get(&thread)
            .expect("kept")
            .claude_session_id,
        None,
        "the dead session is forgotten"
    );

    app.send_chat("typed meanwhile");
    app.agent_says(Notice::Failed {
        error: "claude is not installed".into(),
    });
    let transcript = app.chat().transcript.expect("open");
    assert_eq!(transcript.error.as_deref(), Some("claude is not installed"));
    assert_eq!(
        app.chat().composer.queued.len(),
        1,
        "the follow-up stays queued"
    );
    assert!(app.chat().composer.can_send_empty);
}

#[test]
fn stop_cancels_the_run_and_cancelled_clears_the_run_bar() {
    let mut app = queued();
    app.send_chat("").agent_says(step("Reading"));
    let bar = app
        .chat()
        .transcript
        .and_then(|t| t.run)
        .expect("a run bar");
    assert_eq!(bar.label, "Reading");
    app.take_effects();
    app.act(bar.stop);
    let thread = app.chat_thread_id().expect("open");
    assert_eq!(app.take_effects(), [Effect::CancelAgent(thread)]);
    app.agent_says(Notice::Cancelled);
    assert!(app.chat().transcript.and_then(|t| t.run).is_none());
}

#[test]
fn threads_are_made_opened_left_and_closed() {
    let mut app = queued();
    let first = app.chat_thread_id().expect("the comment's draft");
    app.act(Action::Chat(ChatAction::NewThread));
    let second = app.chat_thread_id().expect("a new thread");
    assert_ne!(first, second);
    let model = app.chat();
    assert_eq!(model.title, "New thread");
    assert_eq!(model.threads.len(), 2);
    assert_eq!(model.threads[0].id, second, "newest first");
    app.act(model.back.expect("a thread is open"));
    assert!(app.chat().transcript.is_none() && app.chat().title == "Threads");
    app.act(app.chat().threads[1].select.clone());
    assert_eq!(app.chat_thread_id(), Some(first.clone()));
    app.take_effects();
    app.act(app.chat().close.expect("a thread is open"));
    assert_eq!(app.chat().threads.len(), 1, "it drops out of the switcher");
    assert!(app.chat_thread_id().is_none());
    assert!(
        app.take_effects()
            .contains(&Effect::WriteThread(first.clone()))
    );
    let file = app
        .app()
        .threads()
        .get(&first)
        .expect("kept on disk")
        .to_json();
    assert!(file.contains("\"status\": \"closed\""), "{file}");
}

#[test]
fn a_thread_cannot_be_closed_while_it_runs() {
    let mut app = queued();
    app.send_chat("");
    app.take_effects();
    app.act(Action::Chat(ChatAction::CloseThread(None)));
    assert_eq!(app.take_effects(), []);
    assert_eq!(app.chat().threads.len(), 1);
}

#[test]
fn a_pasted_image_is_written_before_the_run_and_listed_in_the_request() {
    let mut app = queued();
    let bytes = AssetBytes::from(vec![1, 2, 3]);
    app.send_chat_with(
        "see this",
        vec![ImageUpload {
            media_type: MediaType::Png,
            bytes,
        }],
    );
    let effects = app.take_effects();
    let thread = app.chat_thread_id().expect("open");
    let Some(Effect::WriteAsset { file, .. }) = effects.first() else {
        panic!("the image comes first: {effects:?}");
    };
    assert!(
        file.starts_with(&format!(
            ".specular/threads/tab_1/attachments/{}/img_",
            thread.as_str()
        )) && file.rsplit('.').next() == Some("png"),
        "{file}"
    );
    let [request] = run_requests(&effects).try_into().expect("one run");
    assert_eq!(request.images.len(), 1);
    assert_eq!(&request.images[0].path, file);
    let json = app.app().threads().get(&thread).expect("kept").to_json();
    assert!(
        json.contains(file.as_str()) && json.contains("image/png"),
        "{json}"
    );
}

#[test]
fn loaded_threads_replace_the_store_and_each_canvas_has_its_own() {
    let mut app = TestApp::with_space([("Home", Document::new()), ("Notes", Document::new())]);
    app.with_chat_panel();
    let json = |id: &str, tab: &str| {
        format!(
            r#"{{"id":"{id}","tabId":"{tab}","title":"{id}","status":"open","createdAt":"2026-01-01T00:00:00.000Z","updatedAt":"2026-01-01T00:00:00.000Z","annotationIds":[],"messages":[]}}"#
        )
    };
    let now = "2026-01-01T00:00:00.000Z";
    let threads = ["t1", "t2"]
        .iter()
        .zip(["tab_1", "tab_2"])
        .filter_map(|(id, tab)| specular_interact::Thread::from_json(&json(id, tab), tab, now))
        .collect();
    let index = specular_interact::ThreadIndex {
        active: None,
        by_canvas: [
            ("tab_1".to_owned(), ThreadId("t1".into())),
            ("tab_2".to_owned(), ThreadId("t2".into())),
        ]
        .into(),
    };
    app.send(Event::ThreadsLoaded { threads, index });
    assert_eq!(app.chat().title, "t1");
    app.switch_to("Notes");
    assert_eq!(app.chat().title, "t2");
    assert_eq!(app.chat().threads.len(), 1);
    app.send(Event::SpaceOpened(Box::new(specular_testkit::space([(
        "Home",
        Document::new(),
    )]))));
    assert_eq!(app.app().threads().all(), []);
}

#[test]
fn opening_a_space_with_a_folder_asks_for_its_threads() {
    let mut app = TestApp::empty();
    app.send(Event::SpaceOpened(Box::new(specular_testkit::space([(
        "Home",
        Document::new(),
    )]))));
    assert!(app.take_effects().contains(&Effect::LoadThreads));
}

/// Places a comment on page `p1` (or on the bare canvas) and sends it from the
/// panel's composer.
fn comment_on(app: &mut TestApp, at: (f32, f32), text: &str) {
    app.click(at);
    if at == ON_PAGE {
        app.answer_element(synthetic_element_at(
            CssSize::new(400, 300),
            (100.0, 100.0).into(),
        ));
    }
    app.send_chat(text);
}

const ON_PAGE: (f32, f32) = (200.0, 200.0);
const ON_CANVAS: (f32, f32) = (900.0, 700.0);

#[test]
fn a_comment_on_an_auto_fix_origin_is_sent_the_moment_it_is_placed() {
    let mut app = panel_app(pages(1));
    app.bind("https://example.com", "/scratch/site", true);
    comment_on(&mut app, ON_PAGE, "make it blue");
    let [first] = run_requests(&app.take_effects()).try_into().expect("a run");
    assert_eq!(first.cwd.as_deref(), Some("/scratch/site"));
    assert!(
        first
            .prompt
            .contains("Working directory (linked repo): /scratch/site")
    );

    app.agent_says(Notice::Session("sess-1".into()))
        .agent_says(finished("done"));
    app.take_effects();
    app.tool(Tool::Comment);
    comment_on(&mut app, ON_PAGE, "and bigger");
    let [second] = run_requests(&app.take_effects()).try_into().expect("a run");
    assert_eq!(second.thread, first.thread, "it joins the open thread");
    assert_eq!(second.resume.as_deref(), Some("sess-1"));
    assert_eq!(second.cwd.as_deref(), Some("/scratch/site"));
    let said: Vec<_> = app.app().threads().all()[0]
        .messages
        .iter()
        .map(|m| m.text.as_str())
        .collect();
    assert_eq!(said, ["make it blue", "done", "and bigger"]);
}

#[test]
fn an_auto_fix_comment_placed_during_a_run_is_sent_when_the_run_ends() {
    let mut app = panel_app(pages(1));
    app.bind("https://example.com", "/scratch/site", true);
    comment_on(&mut app, ON_PAGE, "make it blue");
    app.agent_says(Notice::Session("sess-1".into()));
    app.take_effects();

    app.tool(Tool::Comment);
    comment_on(&mut app, ON_PAGE, "and bigger");
    assert!(
        run_requests(&app.take_effects()).is_empty(),
        "a run is in flight"
    );
    assert_eq!(app.chat().composer.queued.len(), 1);
    // Nothing is focused when the run ends, so the drain is aimed by the queued pin.
    app.key(Key::Escape);

    app.agent_says(finished("done"));
    let [drained] = run_requests(&app.take_effects())
        .try_into()
        .expect("the drain");
    assert_eq!(drained.cwd.as_deref(), Some("/scratch/site"));
    assert_eq!(drained.resume.as_deref(), Some("sess-1"));
    assert!(drained.prompt.contains("[User] and bigger"));
}

#[test]
fn canvas_comments_and_comments_on_other_origins_wait_for_send() {
    let rows = [
        ("a canvas comment", ON_CANVAS, "https://example.com"),
        (
            "a comment on a queue-mode origin",
            ON_PAGE,
            "https://example.com",
        ),
        (
            "a comment on an unbound origin",
            ON_PAGE,
            "https://other.dev",
        ),
    ];
    for (name, at, bound) in rows {
        let mut app = panel_app(pages(1));
        let auto = name != "a comment on a queue-mode origin";
        app.bind(bound, "/scratch/site", auto);
        comment_on(&mut app, at, "make it blue");
        assert!(run_requests(&app.take_effects()).is_empty(), "{name}");
        assert_eq!(app.chat().composer.queued.len(), 1, "{name}");
    }
}

#[test]
fn what_a_run_made_or_edited_is_a_card_under_its_answer() {
    let mut app = panel_app([note(
        "plan",
        Rect::new(100.0, 100.0, 300.0, 400.0),
        "plan.md",
    )]);
    app.send_chat("write it up");
    let thread = app.chat_thread_id().expect("the open thread");
    let wrote = |path: &str, created| {
        Notice::Made(Artifact {
            target: ArtifactTarget::File(path.into()),
            created,
        })
    };
    // A file a Document of the canvas shows, and one that is not on it.
    app.agent_says(wrote("/space/plan.md", false))
        .agent_says(wrote("/repo/src/button.css", true));
    // What the run's own CLI calls do to the canvas: the sticky is made,
    // and the move of the Document is not worth a second card.
    let api = |ticket, command| {
        Event::Api(ApiCall {
            ticket,
            canvas: None,
            thread: Some(thread.clone()),
            run: ApiRun::Apply {
                command,
                select: None,
            },
        })
    };
    let made = sticky("recipe", Rect::new(500.0, 100.0, 200.0, 200.0), "Recipe");
    app.send(api(
        1,
        Command::InsertEntity {
            entity: Box::new(made),
            at: 1,
        },
    ));
    app.send(api(
        2,
        Command::SetRect {
            id: app.entity("plan").id.clone(),
            rect: Rect::new(100.0, 600.0, 300.0, 400.0),
        },
    ));
    let making: Vec<String> = (app.chat().transcript.expect("the thread").making.iter())
        .map(|card| card.label.clone())
        .collect();
    assert_eq!(making, ["plan", "button.css", "Recipe"]);

    app.agent_says(finished("done"));
    assert_chat_snapshot!(app, @r#"
    chat available=yes visible=no width=400 title="write it up"
    thread [x] "write it up" 1970-01-01T00:00:00.000Z
    controls back=yes close=yes
    bubble user "write it up"
    bubble agent "done"
      card "plan" Document edited live
      card "button.css" File new
      card "Recipe" Sticky note new live
    composer placeholder="Follow up…" send-empty=no running=no pill=Canvas "Home" folder=Some("space")
    "#);

    // A click on a card selects its item.
    let card = app.chat().transcript.expect("the thread").messages[1].artifacts[2].clone();
    app.act(card.focus.expect("the sticky is on the canvas"));
    assert_eq!(
        app.selection().items(),
        [ItemId::Entity(app.entity("recipe").id.clone())]
    );
}

#[test]
fn the_chosen_model_names_the_chip_is_saved_and_runs_the_next_turn() {
    let mut app = queued();
    assert_eq!(app.chat().composer.model.label, "Sonnet");

    let listed = [
        ("default", "Default (recommended)", "Opus 5.5"),
        (
            "claude-fable-5-1",
            "Fable 5.1",
            "For your toughest challenges",
        ),
        ("sonnet", "Sonnet 5", "Efficient for routine tasks"),
    ]
    .map(|(value, name, description)| Model {
        value: value.into(),
        name: name.into(),
        description: description.into(),
    });
    app.send(Event::Models(ModelsNews::Listed(listed.into())));
    app.send(Event::Models(ModelsNews::Saved("default".into())));
    assert_eq!(
        app.take_effects(),
        vec![],
        "what is loaded is not saved again"
    );
    let chip = app.chat().composer.model;
    assert_eq!(chip.label, "Default");
    let chosen: Vec<_> = chip
        .options
        .iter()
        .map(|o| (o.name.as_str(), o.chosen))
        .collect();
    assert_eq!(
        chosen,
        [
            ("Default (recommended)", true),
            ("Fable 5.1", false),
            ("Sonnet 5", false)
        ]
    );

    app.act(chip.options[1].choose.clone());
    assert_eq!(
        app.take_effects(),
        vec![Effect::SaveAgentModel("claude-fable-5-1".into())]
    );
    assert_eq!(app.chat().composer.model.label, "Fable 5.1");

    app.send_chat("");
    let [request] = run_requests(&app.take_effects())
        .try_into()
        .expect("one run");
    assert_eq!(request.model.as_deref(), Some("claude-fable-5-1"));
}
