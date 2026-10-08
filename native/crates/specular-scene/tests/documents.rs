//! Scene snapshots of Documents: a markdown file's blocks as rows of text in
//! a card, and the card's status line until the text arrives.

use specular_doc::{Entity, FileRef, Kind, Rect};
use specular_interact::{Event, Key, NoteNotice};
use specular_testkit::{TestApp, assert_scene_snapshot};

fn note(id: &str, x: f64) -> Entity {
    let file = FileRef {
        file: format!("{id}.md"),
        ..FileRef::default()
    };
    Entity::new(id, Rect::new(x, 100.0, 400.0, 300.0), Kind::File(file))
}

fn answer(app: &mut TestApp, id: &str, notice: NoteNotice) {
    let file = format!("{id}.md");
    app.send(Event::Note { file, notice });
}

fn showing(markdown: &str) -> TestApp {
    let mut app = TestApp::with_entities([note("doc", 100.0)]);
    answer(&mut app, "doc", NoteNotice::Text(markdown.to_owned()));
    app
}

#[test]
fn headings_paragraphs_and_inline_styles() {
    let app = showing(
        "# Title\n\nSome **bold**, *soft*, ~~gone~~, `code` and a [link](https://x.test).\n\n\
         ## Section\n\n#### Small heading\n\n![diagram](d.png)\n",
    );
    assert_scene_snapshot!(app);
}

#[test]
fn lists_hang_their_markers_and_nest() {
    let app = showing(
        "- one\n- two\n  1. first\n  2. second\n- [ ] todo\n- [x] done\n\n\
         9. loose\n\n10. list\n\n    continued\n",
    );
    assert_scene_snapshot!(app);
}

#[test]
fn quotes_code_and_rules() {
    let app = showing(
        "> quoted *text*\n>\n> second paragraph\n>\n> > nested\n\n\
         ```rust\nfn main() {}\n```\n\n---\n\nafter\n",
    );
    assert_scene_snapshot!(app);
}

#[test]
fn a_table_is_rows_of_equal_columns() {
    let app = showing("| Name | Qty | Note |\n|:--|--:|:-:|\n| **a** | 1 | `x` |\n| b | 22 | |\n");
    assert_scene_snapshot!(app);
}

#[test]
fn a_document_without_its_text_says_why() {
    let mut app = TestApp::with_entities([
        note("loading", 100.0),
        note("missing", 600.0),
        note("failed", 1100.0),
        note("empty", 1600.0),
    ]);
    answer(&mut app, "missing", NoteNotice::Missing);
    answer(&mut app, "failed", NoteNotice::Failed);
    answer(&mut app, "empty", NoteNotice::Text(String::new()));
    assert_scene_snapshot!(app);
}

#[test]
fn a_scrolled_document_keeps_its_rows_and_says_how_far() {
    let mut app = showing("one\n\ntwo\n");
    app.click((300.0, 250.0)).pointer_move((300.0, 250.0));
    app.wheel((0.0, -40.0));
    assert_scene_snapshot!(app);
}

/// The Document `doc` double-clicked on its first character.
fn editing(markdown: &str) -> TestApp {
    let mut app = showing(markdown);
    app.tick(1_000).double_click((113.0, 117.0));
    app
}

#[test]
fn an_edited_document_is_its_source_with_the_syntax_styled_over_it() {
    let mut app = editing(
        "# Title\n\nSome **bold**, *soft* and `code`.\n\n- [ ] a [link](https://x.test)\n\
         ```\nlet x = 1;\n```\n",
    );
    // Into the heading, so the caret stands a heading row tall.
    app.key(Key::ArrowRight).key(Key::ArrowRight);
    assert_scene_snapshot!(app);
}
