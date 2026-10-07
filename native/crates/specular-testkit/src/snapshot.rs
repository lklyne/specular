//! Text snapshots for `insta`.

use std::fmt::Write as _;

use serde_json::Value;
use specular_doc::Document;

/// The document as stable text: what a save would write, one line per node,
/// edge and annotation, in file order.
///
/// ```text
/// nodes:
///   {"id":"p1","type":"link","x":100,"y":100,"width":400,"height":300,"url":"https://example.com/p1"}
/// edges:
/// specular: {"entityOrder":["p1"]}
/// ```
///
/// Keys come out in the writer's order, which relies on `serde_json`'s
/// `preserve_order` feature. `specular-doc` turns it on.
#[track_caller]
pub fn doc_snapshot(document: &Document) -> String {
    let top = match document.to_canvas_value() {
        Ok(Value::Object(top)) => top,
        other => panic!("the document could not be written: {other:?}"),
    };
    let mut out = String::new();
    for (key, value) in top {
        // Writing to a `String` cannot fail.
        let _ = match value {
            Value::Array(items) => {
                let _ = writeln!(out, "{key}:");
                items.iter().try_for_each(|item| writeln!(out, "  {item}"))
            }
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Object(_) => {
                writeln!(out, "{key}: {value}")
            }
        };
    }
    out.truncate(out.trim_end().len());
    out
}

/// Asserts that a [`TestApp`](crate::TestApp)'s document matches its
/// snapshot. Takes what `insta::assert_snapshot!` takes after the value:
///
/// ```ignore
/// assert_doc_snapshot!(app);              // tests/snapshots/<test name>.snap
/// assert_doc_snapshot!(app, @"nodes: …"); // inline
/// assert_doc_snapshot!("after_move", app);
/// ```
#[macro_export]
macro_rules! assert_doc_snapshot {
    ($name:literal, $app:expr $(,)?) => {
        $crate::insta::assert_snapshot!($name, $app.doc_snapshot())
    };
    ($app:expr $(, $($rest:tt)*)?) => {
        $crate::insta::assert_snapshot!($app.doc_snapshot() $(, $($rest)*)?)
    };
}
