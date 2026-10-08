//! What the pill points at, how it reads, and the focus line it gives a prompt.

use specular_agent::{
    CanvasSelection, FocusedAnnotation, InspectNode, Pill, PillInput, focus_prompt, resolve,
};

fn node(name: &str, tag: &str) -> InspectNode {
    InspectNode {
        name: name.into(),
        tag_name: tag.into(),
        origin: Some("http://a.test".into()),
        page_id: Some("p1".into()),
    }
}

fn comment(text: &str, element: Option<&str>, anchor: &str) -> FocusedAnnotation {
    FocusedAnnotation {
        id: "c1".into(),
        text: text.into(),
        element_name: element.map(Into::into),
        anchor_type: anchor.into(),
    }
}

fn selection(count: usize) -> CanvasSelection {
    CanvasSelection {
        count,
        label: format!("{count} items"),
        entity_ids: vec!["a".into(), "b".into()],
    }
}

fn label(input: &PillInput) -> String {
    resolve(input).label("Home")
}

#[test]
fn the_pill_prefers_a_dom_node_then_a_comment_then_a_selection() {
    let all = PillInput {
        inspect_node: Some(node("Buy", "button")),
        focused_annotation: Some(comment("text", None, "page")),
        canvas_selection: Some(selection(2)),
    };
    assert!(matches!(resolve(&all), Pill::Dom { .. }));
    assert_eq!(label(&all), "Buy");

    let no_node = PillInput {
        inspect_node: None,
        ..all.clone()
    };
    assert_eq!(
        resolve(&no_node),
        Pill::Annotation {
            label: "text".into(),
            annotation_id: "c1".into()
        }
    );

    let only_selection = PillInput {
        focused_annotation: None,
        ..no_node.clone()
    };
    assert!(matches!(resolve(&only_selection), Pill::Selection { .. }));

    let zero = PillInput {
        canvas_selection: Some(selection(0)),
        ..PillInput::default()
    };
    assert_eq!(resolve(&zero), Pill::Empty);
}

#[test]
fn pill_labels_fall_back_in_order() {
    let dom = |n, t| PillInput {
        inspect_node: Some(node(n, t)),
        ..PillInput::default()
    };
    let note = |text: &str, el: Option<&str>, anchor: &str| PillInput {
        focused_annotation: Some(comment(text, el, anchor)),
        ..PillInput::default()
    };
    let long = "word ".repeat(20);
    let cases = [
        (dom("  ", "div"), "div".to_owned()),
        (dom(" ", " "), "element".to_owned()),
        (note("hi", Some(" nav "), "page"), "nav".to_owned()),
        (note("  two\n words ", None, "page"), "two words".to_owned()),
        (
            note(&long, None, "page"),
            format!("{}…", &long.trim()[..47]),
        ),
        (note("", None, "element"), "Element".to_owned()),
        (note("", None, "region"), "Region".to_owned()),
        (note("", None, "page"), "Page".to_owned()),
        (note("", None, "weird"), "Comment".to_owned()),
        (PillInput::default(), "Home".to_owned()),
    ];
    for (input, expected) in cases {
        assert_eq!(label(&input), expected);
    }
    assert_eq!(Pill::Empty.label("  "), "specular");
}

#[test]
fn the_focus_prompt_names_what_is_picked() {
    let dom = Pill::Dom {
        label: "Buy".into(),
        origin: None,
        page_id: Some("p1".into()),
    };
    let bare = Pill::Dom {
        label: "Buy".into(),
        origin: None,
        page_id: None,
    };
    let sel = Pill::Selection {
        label: "2 items".into(),
        entity_ids: vec!["a".into(), "b".into()],
    };
    let empty_sel = Pill::Selection {
        label: "x".into(),
        entity_ids: vec![],
    };
    let note = Pill::Annotation {
        label: "x".into(),
        annotation_id: "c1".into(),
    };
    assert_eq!(
        focus_prompt(&dom).unwrap(),
        "The user has selected DOM node \"Buy\" on page p1 and likely wants to focus on that."
    );
    assert_eq!(
        focus_prompt(&bare).unwrap(),
        "The user has selected DOM node \"Buy\" and likely wants to focus on that."
    );
    assert_eq!(
        focus_prompt(&sel).unwrap(),
        "The user has selected a, b and likely wants to focus on those."
    );
    assert_eq!(
        focus_prompt(&note).unwrap(),
        "The user has selected comment c1 and likely wants to focus on that."
    );
    assert_eq!(focus_prompt(&empty_sel), None);
    assert_eq!(focus_prompt(&Pill::Empty), None);
}
