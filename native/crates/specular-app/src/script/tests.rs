use std::path::PathBuf;

use glam::Vec2;
use serde_json::json;
use specular_core::{Camera, Modifiers};
use specular_interact::{Action, ArrangeMode, Key, Tool};

use super::words::NO_MODIFIERS;
use super::*;

fn steps(text: &str) -> Vec<Step> {
    let script = Script::parse(text).unwrap();
    script.steps.into_iter().map(|line| line.step).collect()
}

#[test]
fn lines_parse_into_steps() {
    let held = Modifiers {
        shift: true,
        meta: true,
        ..NO_MODIFIERS
    };
    let at = CameraArg::At(Camera::new(Vec2::new(40.0, -20.0), 0.5));
    for (script, expected) in [
        (
            "# a marquee\n\n  press 10 20\nrelease\nrelease 5 6\n",
            vec![
                Step::Press(Vec2::new(10.0, 20.0)),
                Step::Release,
                Step::ReleaseAt(Vec2::new(5.0, 6.0)),
            ],
        ),
        (
            "key cmd+shift+z\nkey escape",
            vec![
                Step::Key(held, Key::Char('z')),
                Step::Key(NO_MODIFIERS, Key::Escape),
            ],
        ),
        (
            "type Hello,  world",
            vec![Step::Type("Hello,  world".to_owned())],
        ),
        (
            r"clipboard one\ntwo\\n",
            vec![Step::Clipboard("one\ntwo\\n".to_owned())],
        ),
        (
            "act annotate-selection\nact arrange-row\nact undo",
            vec![
                Step::Act(Action::AnnotateSelection),
                Step::Act(Action::Arrange(ArrangeMode::Row)),
                Step::Act(Action::Undo),
            ],
        ),
        (
            "camera fit\ncamera 40,-20,0.5",
            vec![Step::Camera(CameraArg::Fit), Step::Camera(at)],
        ),
        (
            "scroll 1 2 3 4\nshot a.png\nsnapshot b.png\nstate s.txt",
            vec![
                Step::ScrollAt(Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0)),
                Step::Snapshot(PathBuf::from("a.png")),
                Step::Snapshot(PathBuf::from("b.png")),
                Step::State(PathBuf::from("s.txt")),
            ],
        ),
        (
            "mark arranged\nreopen\nundo-to-start",
            vec![
                Step::Mark("arranged".to_owned()),
                Step::Reopen,
                Step::UndoToStart,
            ],
        ),
        (
            "tool select\ntool page\ntool text\ntool sticky\ntool document\ntool shape\n\
             tool draw\ntool comment\ntool inspect",
            [
                Tool::Select,
                Tool::AddPage,
                Tool::AddText,
                Tool::AddSticky,
                Tool::AddDocument,
                Tool::AddShape,
                Tool::Draw,
                Tool::Comment,
                Tool::Inspect,
            ]
            .map(Step::Tool)
            .to_vec(),
        ),
    ] {
        assert_eq!(steps(script), expected, "{script:?}");
    }
}

#[test]
fn window_steps_parse_and_are_marked_as_the_window_s() {
    let window = |step| Step::Window(step);
    for (text, expected) in [
        (
            "resize 1200 800",
            window(WindowStep::Resize(Vec2::new(1200.0, 800.0))),
        ),
        (
            "keys shift+cmd 6 z",
            window(WindowStep::KeyCode {
                code: 6,
                held: Modifiers {
                    shift: true,
                    meta: true,
                    ..NO_MODIFIERS
                },
                chars: Some("z".to_owned()),
            }),
        ),
        (
            "drop /a.png nowhere",
            window(WindowStep::Drop(vec![PathBuf::from("/a.png")], None)),
        ),
        (
            "swipe 0 -200 40 400",
            window(WindowStep::Swipe(Vec2::new(0.0, -200.0), 40, 400)),
        ),
        (
            "menu-choose Edit > Undo",
            window(WindowStep::MenuChoose("Edit > Undo".to_owned())),
        ),
        ("quit", window(WindowStep::Quit)),
    ] {
        assert_eq!(steps(text), vec![expected], "{text}");
    }
}

#[test]
fn expect_lines_read_as_facts() {
    let expect = |text: &str| match steps(text).pop() {
        Some(Step::Expect(expect)) => expect,
        other => panic!("{text}: {other:?}"),
    };
    let cases = [
        ("expect tool sticky", Expect::Tool(Tool::AddSticky)),
        ("expect selection none", Expect::Selection(vec![])),
        (
            "expect selection a b",
            Expect::Selection(vec!["a".to_owned(), "b".to_owned()]),
        ),
        ("expect focus page p1", Expect::Focus(Some("p1".to_owned()))),
        (
            r"expect editing two\nlines",
            Expect::Editing(Some("two\nlines".to_owned())),
        ),
        ("expect undo none", Expect::Undo(false)),
        (
            "expect count sticky 2",
            Expect::Count(Counted::Kind("sticky".to_owned()), 2),
        ),
        (
            "expect entity @top rect 1 2 3 4",
            Expect::Entity("@top".to_owned(), EntityFact::Rect([1.0, 2.0, 3.0, 4.0])),
        ),
        (
            "expect entity s1 text Ship the beta",
            Expect::Entity(
                "s1".to_owned(),
                EntityFact::Text("Ship the beta".to_owned()),
            ),
        ),
        (
            r#"expect saved /nodes/s1/color = "4""#,
            Expect::Saved("/nodes/s1/color".to_owned(), SavedFact::Equals(json!("4"))),
        ),
        (
            "expect saved /nodes/s1/text = first note",
            Expect::Saved(
                "/nodes/s1/text".to_owned(),
                SavedFact::Equals(json!("first note")),
            ),
        ),
        (
            "expect saved same-as mark arranged",
            Expect::SameAs(Reference::Mark("arranged".to_owned())),
        ),
        (
            "expect saved changed-from start at /nodes/a/x /nodes/a/y",
            Expect::ChangedFrom(
                Reference::Start,
                vec!["/nodes/a/x".to_owned(), "/nodes/a/y".to_owned()],
            ),
        ),
    ];
    for (text, wanted) in cases {
        assert_eq!(expect(text), wanted, "{text}");
    }
}

#[test]
fn given_lines_build_the_starting_document_and_come_first() {
    let script = Script::parse(
        "# canvas: empty.canvas\ngiven group g 0 0 900 900\ngiven inside g sticky s 10 20 200 200 Hi there\n\
         given shape r 400 20 100 100\ngiven edge e s r\nclick 1 1",
    )
    .unwrap();
    assert_eq!(script.canvas.as_deref(), Some("empty.canvas"));
    let document = script.given_document().unwrap().unwrap();
    assert_eq!(document.entities().count(), 3);
    assert_eq!(document.edges().count(), 1);
    let sticky = document.entity(&"s".into()).unwrap();
    assert_eq!(
        sticky.parent.as_ref().map(ToString::to_string).as_deref(),
        Some("g")
    );
    assert!(Script::parse("click 1 1\ngiven sticky s 0 0 1 1").is_err());
}

#[test]
fn bad_lines_are_rejected_with_their_number() {
    for script in [
        "act nonsense",
        "panels maybe",
        "sidebar",
        "control",
        "tool hammer",
        "key f13",
        "hold hyper",
        "expect wishes",
        "expect entity s1 colour red",
        "resize 10",
        "given blob b 0 0 1 1",
    ] {
        assert!(Script::parse(script).is_err(), "{script:?}");
    }
    let error = Script::parse("click 1 2\nclick 1").unwrap_err();
    assert!(format!("{error:#}").starts_with("line 2"));
}
