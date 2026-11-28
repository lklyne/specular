//! The command palette mod against `panels/palette`: the commands a query
//! finds, in the model's order, and a click on one running it.

use gpui::TestAppContext;
use serde_json::{Value, json};
use specular_interact::{Action, Tool, state};
use specular_testkit::TestApp;

use crate::support::{Loaded, grid, timed_load, window_app};

const SIZE: (f32, f32) = (560.0, 480.0);
/// The rows the mod shows at most.
const SHOWN: usize = 12;

/// Two pages, the first selected, so the edit and arrange commands are on.
fn app() -> TestApp {
    let mut app = window_app();
    app.select(&["p1"]);
    app
}

/// The palette, opened as its overlay opens it, with `query` typed.
fn opened(app: &TestApp, props: Value, query: &str, cx: &mut TestAppContext) -> Loaded {
    let mut loaded = Loaded::new(app, "palette", props, SIZE, cx);
    let field = field(&loaded);
    (loaded.hosting.revisions.borrow_mut()).publish(&format!("shared/field/{field}"), json!(query));
    loaded.step(app);
    loaded
}

/// The id of the palette's query field, which it makes when it opens.
fn field(loaded: &Loaded) -> String {
    (loaded.reads("palette").into_iter())
        .find_map(|key| {
            key.strip_prefix("shared/field/")
                .filter(|rest| !rest.ends_with("/submit"))
                .map(str::to_owned)
        })
        .expect("the palette reads its field")
}

/// The `(group, label)` of each command the model lists that `query`
/// finds, but those `hidden`, as many as the palette shows.
///
/// GAP: an enabled item whose `command` is null is left out, since the mod
/// has nothing to run for it: the Tools group's "Shape"
/// (`SetToolVariant(ShapeKind(Rectangle))`) and "Draw"
/// (`SetToolVariant(Brush(Pen))`), which no registry command names. The
/// native palette also lists disabled items, greyed; the mod lists only
/// the enabled ones.
fn listed(palette: &Value, query: &str, hidden: &[&str]) -> Vec<(String, String)> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    (palette.as_array().into_iter().flatten())
        .filter(|item| item["enabled"] == true)
        .filter(|item| {
            item["command"]
                .as_str()
                .is_some_and(|command| !hidden.contains(&command))
        })
        .filter(|item| {
            let keywords =
                (item["keywords"].as_array().into_iter().flatten()).filter_map(Value::as_str);
            let text = [item["label"].as_str(), item["group"].as_str()]
                .into_iter()
                .flatten()
                .chain(keywords)
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            words.iter().all(|word| text.contains(word.as_str()))
        })
        .map(|item| {
            let text = |key: &str| item[key].as_str().unwrap_or_default().to_owned();
            (text("group"), text("label"))
        })
        .take(SHOWN)
        .collect()
}

/// The `(group, label)` of each row the tree shows, top to bottom.
fn rows(tree: &str) -> Vec<(String, String)> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in tree.lines().map(str::trim) {
        if line.starts_with("Button \"palette.") {
            rows.push(Vec::new());
        } else if let (Some(row), Some(text)) = (rows.last_mut(), line.strip_prefix("text \"")) {
            row.push(text.trim_end_matches('"').to_owned());
        }
    }
    (rows.into_iter())
        .map(|row| {
            let mut texts = row.into_iter();
            (
                texts.next().unwrap_or_default(),
                texts.next().unwrap_or_default(),
            )
        })
        .collect()
}

#[gpui::test]
fn the_palette_shows_every_command_a_query_finds_but_those_hidden(cx: &mut TestAppContext) {
    let app = app();
    let palette = state(app.app())["panels/palette"].clone();
    let first = (palette.as_array().into_iter().flatten())
        .find(|item| item["enabled"] == true)
        .and_then(|item| item["command"].as_str())
        .expect("an enabled command")
        .to_owned();
    for query in [
        "",
        "canvas",
        "zoom",
        "layout",
        "select all",
        "no such command",
    ] {
        for hidden in [&[][..], &[first.as_str(), "app.settings"][..]] {
            let props = json!({ "hidden": hidden, "overlay": "palette" });
            let tree = opened(&app, props, query, cx).tree();
            let wanted = listed(&palette, query, hidden);
            assert_eq!(
                rows(&tree),
                wanted,
                "`{query}` with {hidden:?} hidden:\n{tree}"
            );
            if wanted.is_empty() {
                assert!(tree.contains("No command matches"), "{tree}");
            }
        }
    }
    assert!(
        listed(&palette, "", &[]).len() == SHOWN && !listed(&palette, "layout", &[]).is_empty(),
        "the queries find commands"
    );
}

#[gpui::test]
fn a_click_on_a_command_runs_it_through_the_host(cx: &mut TestAppContext) {
    let app = app();
    for (query, command) in [
        ("settings", "app.settings"),
        ("zoom in", "view.zoom-in"),
        ("select all", "edit.select-all"),
    ] {
        let mut loaded = opened(&app, json!({ "overlay": "palette" }), query, cx);
        let runs = loaded.click_until_run(grid((560, 200), 6), |run| {
            run.starts_with(&format!("{command} "))
        });
        assert!(
            runs.iter()
                .any(|run| run.starts_with(&format!("{command} "))),
            "`{query}` runs {command}: {runs:?}"
        );
    }
}

#[gpui::test]
fn the_palette_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = app();
    let mut loaded = timed_load(&app, "palette", json!({ "overlay": "palette" }), SIZE, cx);
    let field = field(&loaded);
    let mut reads = loaded.reads("palette");
    reads.sort();
    assert_eq!(
        reads,
        [
            "look".to_owned(),
            "mods/palette/props".to_owned(),
            "panels/palette".to_owned(),
            format!("shared/field/{field}"),
            format!("shared/field/{field}/submit"),
        ]
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    loaded.assert_camera_moves_nothing(&mut app);
    crate::support::assert_renders_inside_a_frame(
        &mut loaded,
        &mut app,
        "palette",
        300,
        |app, i| {
            // The tool in hand is checked in the Tools group.
            let tool = if i % 2 == 0 {
                Tool::AddText
            } else {
                Tool::Select
            };
            app.act(Action::SetTool(tool));
        },
    );
}

#[gpui::test]
fn the_palette_runs_the_command_typed_on_return(cx: &mut TestAppContext) {
    let app = app();
    let mut loaded = Loaded::new(
        &app,
        "palette",
        json!({ "overlay": "palette" }),
        (560.0, 480.0),
        cx,
    );
    let field = (loaded.reads("palette").into_iter())
        .find_map(|key| {
            key.strip_prefix("shared/field/")
                .and_then(|rest| rest.strip_suffix("/submit"))
                .map(str::to_owned)
        })
        .expect("the palette reads its field's submit");
    let publish = |loaded: &Loaded, key: String, value: serde_json::Value| {
        loaded.hosting.revisions.borrow_mut().publish(&key, value);
    };
    publish(&loaded, format!("shared/field/{field}"), json!("settings"));
    loaded.step(&app);
    publish(&loaded, format!("shared/field/{field}/submit"), json!(1));
    loaded.step(&app);
    let runs = loaded.runs();
    assert!(
        runs.iter().any(|run| run.starts_with("app.settings")),
        "{runs:?}"
    );
}
