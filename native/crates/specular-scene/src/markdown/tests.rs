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
fn inline_markup_becomes_spans_with_the_markers_gone() {
    assert_eq!(
        outline("a **b** *c* ~~d~~ `e` [f](https://x.test) ***g***"),
        "p a {s:b} {e:c} {x:d} {c:e} {l:f} {se:g}"
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
