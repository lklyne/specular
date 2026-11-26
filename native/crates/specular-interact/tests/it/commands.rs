//! The command registry is well formed, and running by name refuses what
//! it cannot run.

use std::collections::HashSet;

use serde_json::{Value, json};
use specular_interact::{Action, Run, RunError, Tool, commands, run_named};
use specular_testkit::TestApp;

#[test]
fn every_command_is_named_once_in_dotted_lower_case_and_builds_bare_when_it_takes_nothing() {
    let mut seen = HashSet::new();
    for spec in commands() {
        let name = spec.name;
        assert!(seen.insert(name), "{name} is in the registry twice");
        let (area, verb) = name.split_once('.').unwrap_or_default();
        let word = |part: &str| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        };
        assert!(word(area) && word(verb), "{name} is not dotted lower case");
        assert!(!spec.about.is_empty(), "{name} has no about");
        if spec.args.is_empty() {
            assert!(
                (spec.build)(&Value::Null).is_ok(),
                "{name} takes nothing and does not build"
            );
        }
    }
}

#[test]
fn running_by_name_builds_with_args_and_refuses_what_it_cannot_run() {
    let app = TestApp::empty();
    let app = app.app();
    let rows: [(&str, Value, Result<Run, &str>); 7] = [
        (
            "tool.set",
            json!({ "tool": "draw" }),
            Ok(Run::Action(Action::SetTool(Tool::Draw))),
        ),
        (
            "view.zoom",
            json!({ "percent": 50 }),
            Ok(Run::Action(Action::ZoomTo(50))),
        ),
        (
            "tool.set",
            json!({ "tool": "hand" }),
            Err("`tool.set` takes {tool:"),
        ),
        (
            "item.add",
            json!({ "kind": "page", "x": 0, "y": 0 }),
            Err("`item.add` takes {kind: sticky|text|shape"),
        ),
        ("edit.undoo", Value::Null, Err("did you mean edit.undo")),
        ("edit.undo", Value::Null, Err("`edit.undo` cannot run now")),
        (
            "control.press",
            json!({ "id": "no.such" }),
            Err("no control `no.such`"),
        ),
    ];
    for (name, args, want) in rows {
        let got = run_named(app, name, &args);
        match want {
            Ok(run) => assert_eq!(got, Ok(run), "{name} {args}"),
            Err(text) => {
                let error = got
                    .err()
                    .map_or_else(String::new, |error: RunError| error.to_string());
                assert!(
                    error.contains(text),
                    "{name} {args}: {error:?} lacks {text:?}"
                );
            }
        }
    }
}
