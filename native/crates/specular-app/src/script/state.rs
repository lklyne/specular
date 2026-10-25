//! The state an `expect` reads, as text in its own words: each line is an
//! `expect` without the word, so a line of a dump can be pasted into a
//! script as a check.

use std::fmt::Write as _;

use specular_interact::{App, Focus, Showing};

use super::check::{kind_word, text_of};
use super::words::{escaped, tool_word};

/// The app's state, one fact a line.
pub fn dump(app: &App) -> String {
    let session = app.session();
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };
    line(format!("canvas {}", app.space().active().name));
    line(format!("count canvases {}", app.space().canvases().len()));
    line(format!("tool {}", tool_word(session.tool)));
    let selection: Vec<&str> = session
        .selection
        .items()
        .iter()
        .map(|item| item.as_str())
        .collect();
    line(if selection.is_empty() {
        "selection none".to_owned()
    } else {
        format!("selection {}", selection.join(" "))
    });
    line(match &session.focus {
        Focus::Canvas => "focus canvas".to_owned(),
        Focus::Page(id) => format!("focus page {id}"),
    });
    line(match app.text_edit() {
        Some(edit) => format!("editing {}", escaped(edit.text())),
        None => "editing none".to_owned(),
    });
    line(match app.showing() {
        Showing::Canvas => "showing canvas".to_owned(),
        Showing::Item(id) => format!("showing {id}"),
    });
    let shown = if session.sidebar.shown() { "on" } else { "off" };
    line(format!("sidebar {shown}"));
    let open = session.panel.menu.is_some() || session.panel.open.is_some();
    line(format!("menu {}", if open { "open" } else { "closed" }));
    let some = |yes: bool| if yes { "some" } else { "none" };
    line(format!("undo {}", some(app.can_undo())));
    line(format!("redo {}", some(app.can_redo())));
    let document = app.document();
    line(format!("count entities {}", document.entities().count()));
    line(format!("count edges {}", document.edges().count()));
    line(format!("count comments {}", document.annotations().len()));
    for entity in document.entities() {
        let (id, rect) = (&entity.id, entity.rect);
        line(format!("entity {id} kind {}", kind_word(entity)));
        line(format!(
            "entity {id} rect {} {} {} {}",
            rect.x, rect.y, rect.width, rect.height
        ));
        let text = text_of(entity);
        if !text.is_empty() {
            line(format!("entity {id} text {}", escaped(text)));
        }
        if let Some(parent) = &entity.parent {
            line(format!("entity {id} parent {parent}"));
        }
    }
    let camera = session.camera;
    let _ = writeln!(
        out,
        "# camera {:.1},{:.1},{:.3}",
        camera.pan.x, camera.pan.y, camera.zoom
    );
    out
}
