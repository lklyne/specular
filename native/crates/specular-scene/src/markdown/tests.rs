use std::fmt::Write as _;

use super::{Block, BlockKind, ColumnAlign, Inline, Marker, parse};

/// The blocks as text, one a line: `>` per quote, two spaces per list
/// depth, `~` for a tight block, the kind, then the text with each styled
/// stretch wrapped in `{flags:…}`. The flags are the first letters of
/// strong, emphasis, strike (`x`), code, link and image.
fn outline(markdown: &str) -> String {
    let lines: Vec<String> = parse(markdown).iter().map(line).collect();
    lines.join("\n")
}

fn line(block: &Block) -> String {
    let mut out = ">".repeat(usize::from(block.quote));
    out.push_str(&"  ".repeat(usize::from(block.indent)));
    if block.tight {
        out.push('~');
    }
    // Writing to a `String` cannot fail.
    let _ = match &block.kind {
        BlockKind::Heading { level, text } => write!(out, "h{level} {}", marked(text)),
        BlockKind::Paragraph(text) => write!(out, "p {}", marked(text)),
        BlockKind::Item { marker, text } => {
            let marker = match marker {
                Marker::Bullet => "-".to_owned(),
                Marker::Number(number) => format!("{number}."),
                Marker::Task { done: false } => "[ ]".to_owned(),
                Marker::Task { done: true } => "[x]".to_owned(),
            };
            write!(out, "{marker} {}", marked(text))
        }
        BlockKind::Code(code) => write!(out, "code {code:?}"),
        BlockKind::Rule => write!(out, "rule"),
        BlockKind::Table(table) => {
            let columns: String = (table.columns.iter())
                .map(|column| match column {
                    ColumnAlign::Left => 'l',
                    ColumnAlign::Centre => 'c',
                    ColumnAlign::Right => 'r',
                })
                .collect();
            let row = |cells: &[Inline]| cells.iter().map(marked).collect::<Vec<_>>().join(" | ");
            let rows: Vec<String> = table.rows.iter().map(|cells| row(cells)).collect();
            write!(
                out,
                "table {columns} [{}] [{}]",
                row(&table.head),
                rows.join("] [")
            )
        }
    };
    out
}

fn marked(inline: &Inline) -> String {
    let mut out = String::new();
    let mut at = 0;
    for span in &inline.spans {
        out.push_str(&inline.text[at..span.range.start]);
        let style = span.style;
        let flags: String = [
            (style.strong, 's'),
            (style.emphasis, 'e'),
            (style.strike, 'x'),
            (style.code, 'c'),
            (style.link, 'l'),
            (style.image, 'i'),
        ]
        .iter()
        .filter_map(|&(on, flag)| on.then_some(flag))
        .collect();
        let _ = write!(out, "{{{flags}:{}}}", &inline.text[span.range.clone()]);
        at = span.range.end;
    }
    out.push_str(&inline.text[at..]);
    out
}

#[test]
fn headings_and_paragraphs_are_blocks_in_order() {
    let outline = outline("# Title\n\nFirst\nline.\n\n### Deep\n\nSecond.  \nBroken.\n");
    assert_eq!(
        outline,
        "h1 Title\np First line.\nh3 Deep\np Second.\nBroken."
    );
}

#[test]
fn inline_markup_becomes_spans_with_the_markers_gone() {
    assert_eq!(
        outline("a **b** *c* ~~d~~ `e` [f](https://x.test) ***g***"),
        "p a {s:b} {e:c} {x:d} {c:e} {l:f} {se:g}"
    );
}

#[test]
fn styles_nest_and_neighbours_in_one_style_merge() {
    assert_eq!(
        outline("**bold `code` and [link *em*](u)** end"),
        "p {s:bold }{sc:code}{s: and }{sl:link }{sel:em} end"
    );
}

#[test]
fn a_tight_list_keeps_its_items_together_after_the_first() {
    assert_eq!(
        outline("Intro\n\n- one\n- two\n- three\n"),
        "p Intro\n  - one\n  ~- two\n  ~- three"
    );
}

#[test]
fn nested_lists_carry_their_depth() {
    assert_eq!(
        outline("- a\n  - b\n    1. c\n  - d\n- e\n"),
        "  - a\n    ~- b\n      ~1. c\n    ~- d\n  ~- e"
    );
}

#[test]
fn block_quotes_carry_their_depth_through_what_they_hold() {
    assert_eq!(
        outline("> quoted\n>\n> - item\n>\n> > deeper\n\nout\n"),
        ">p quoted\n>  - item\n>>p deeper\np out"
    );
}

#[test]
fn code_blocks_keep_their_text_exactly() {
    assert_eq!(
        outline("```rust\nfn main() {\n    let a = *b*;\n}\n```\n\n    indented\n"),
        "code \"fn main() {\\n    let a = *b*;\\n}\"\ncode \"indented\""
    );
}

#[test]
fn tables_keep_alignment_head_and_rows() {
    let table = "| Name | Qty | Note |\n|:--|--:|:-:|\n| **a** | 1 | `x` |\n| b | 22 | |\n";
    assert_eq!(
        outline(table),
        "table lrc [Name | Qty | Note] [{s:a} | 1 | {c:x}] [b | 22 | ]"
    );
}

#[test]
fn spans_stay_on_character_boundaries_with_wide_text() {
    for block in parse("café **naïve** 日本語 *文字* ~~x~~ `ø`") {
        let BlockKind::Paragraph(text) = block.kind else {
            panic!("not a paragraph");
        };
        assert_eq!(text.text, "café naïve 日本語 文字 x ø");
        for span in &text.spans {
            assert!(text.text.get(span.range.clone()).is_some(), "{span:?}");
        }
    }
}
