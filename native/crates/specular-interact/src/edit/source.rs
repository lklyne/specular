//! Markdown source as it is shown while edited: every character stays where
//! it was typed, and the syntax is styled over it. A heading line is larger
//! and heavy, emphasis is set as it will read, and the markers themselves
//! are faint.
//!
//! This is a styler, not a parser. It reads one line at a time, with a code
//! fence as the only thing carried from line to line, so a keystroke restyles
//! the line it lands on and nothing shifts under the caret.

use std::ops::Range;

/// How a stretch of markdown source is set.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the styles are independent and nest in any combination"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceStyle {
    /// Strong text and headings: set heavy.
    pub strong: bool,
    /// Emphasis: set italic.
    pub emphasis: bool,
    /// Inline code and fenced code: set in the monospace face.
    pub code: bool,
    /// Struck-through text.
    pub strike: bool,
    /// A link's text.
    pub link: bool,
    /// Syntax, not prose: a marker, a fence or a link's target. Set faint.
    pub faint: bool,
}

/// A stretch of one source line set in its own [`SourceStyle`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    /// The bytes of the line it covers.
    pub range: Range<usize>,
    /// How it is set.
    pub style: SourceStyle,
}

/// One line of markdown source, styled.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceLine {
    /// The heading level, 1 to 6, or 0 for a line that is not a heading.
    pub heading: u8,
    /// The stretches set differently from plain text, in order and not
    /// overlapping, on character boundaries.
    pub spans: Vec<SourceSpan>,
}

/// Styles every line of `text`, split at its line breaks.
pub fn style_lines(text: &str) -> Vec<SourceLine> {
    let mut fence: Option<&str> = None;
    text.split('\n')
        .map(|line| style_line(line, &mut fence))
        .collect()
}

/// The fence a line opens or closes: three or more backticks or tildes after
/// at most three spaces.
fn fence_of(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    ["```", "~~~"]
        .into_iter()
        .find(|mark| trimmed.starts_with(mark))
}

fn style_line(line: &str, fence: &mut Option<&'static str>) -> SourceLine {
    let mut styles = vec![SourceStyle::default(); line.len()];
    let code = SourceStyle {
        code: true,
        ..SourceStyle::default()
    };
    match (*fence, fence_of(line)) {
        (Some(open), found) => {
            let closes = found == Some(open) && line.trim().chars().all(|c| open.starts_with(c));
            paint(&mut styles, 0..line.len(), faint_if(code, closes));
            if closes {
                *fence = None;
            }
            return finish(0, &styles);
        }
        (None, Some(open)) => {
            *fence = Some(open);
            paint(&mut styles, 0..line.len(), faint_if(code, true));
            return finish(0, &styles);
        }
        (None, None) => {}
    }
    if is_rule(line) {
        let faint = faint_if(SourceStyle::default(), true);
        paint(&mut styles, 0..line.len(), faint);
        return finish(0, &styles);
    }
    let (at, heading) = block_markup(line, &mut styles);
    let base = SourceStyle {
        strong: heading > 0,
        ..SourceStyle::default()
    };
    inline(line, at..line.len(), base, &mut styles);
    finish(heading, &styles)
}

fn faint_if(style: SourceStyle, faint: bool) -> SourceStyle {
    SourceStyle { faint, ..style }
}

fn paint(styles: &mut [SourceStyle], range: Range<usize>, style: SourceStyle) {
    if let Some(stretch) = styles.get_mut(range) {
        stretch.fill(style);
    }
}

/// A thematic break: three or more of one of `-`, `*` or `_`, with nothing
/// else on the line but spaces.
fn is_rule(line: &str) -> bool {
    let mut marks = line.chars().filter(|c| *c != ' ');
    let Some(first) = marks.next() else {
        return false;
    };
    matches!(first, '-' | '*' | '_') && marks.clone().all(|c| c == first) && marks.count() >= 2
}

/// Paints the markup at the head of `line` faint: quote marks, a heading
/// mark, a list marker and a task box. Returns where the prose starts and
/// the heading level.
fn block_markup(line: &str, styles: &mut [SourceStyle]) -> (usize, u8) {
    let faint = faint_if(SourceStyle::default(), true);
    let bytes = line.as_bytes();
    let spaces = |from: usize| from + bytes[from..].iter().take_while(|b| **b == b' ').count();
    let mut at = spaces(0);
    while bytes.get(at) == Some(&b'>') {
        paint(styles, at..at + 1, faint);
        at = spaces(at + 1);
    }
    let hashes = bytes[at..].iter().take_while(|b| **b == b'#').count();
    let after = bytes.get(at + hashes);
    if (1..=6).contains(&hashes) && (after.is_none() || after == Some(&b' ')) {
        let strong = SourceStyle {
            strong: true,
            ..faint
        };
        paint(styles, at..at + hashes, strong);
        return (spaces(at + hashes), hashes as u8);
    }
    let digits = bytes[at..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    let marker = match bytes.get(at) {
        Some(b'-' | b'*' | b'+') => Some(at + 1),
        Some(_) if digits > 0 && matches!(bytes.get(at + digits), Some(b'.' | b')')) => {
            Some(at + digits + 1)
        }
        _ => None,
    };
    if let Some(end) = marker
        && bytes.get(end) == Some(&b' ')
    {
        paint(styles, at..end, faint);
        at = end + 1;
        let boxed = matches!(line.get(at..at + 4), Some("[ ] " | "[x] " | "[X] "));
        if boxed {
            paint(styles, at..at + 3, faint);
            at += 4;
        }
    }
    (at, 0)
}

/// An inline marker and the style it turns on.
type Marker = (&'static str, fn(&mut SourceStyle));

/// The inline markers, longest first so `**` is tried before `*`.
const MARKERS: [Marker; 6] = [
    ("**", |style| style.strong = true),
    ("__", |style| style.strong = true),
    ("~~", |style| style.strike = true),
    ("*", |style| style.emphasis = true),
    ("_", |style| style.emphasis = true),
    ("`", |style| style.code = true),
];

/// Styles the prose in `range`: code spans, strong, emphasis, strike and
/// links, each closed on the same line. Anything left open is plain text.
fn inline(line: &str, range: Range<usize>, base: SourceStyle, styles: &mut [SourceStyle]) {
    paint(styles, range.clone(), base);
    let mut at = range.start;
    while at < range.end {
        let rest = &line[at..range.end];
        if let Some(end) = link(line, at, range.end, base, styles) {
            at = end;
            continue;
        }
        let found = MARKERS.iter().find_map(|(marker, apply)| {
            let inner = at + marker.len();
            let close = closing(line, marker, at, range.end)?;
            rest.starts_with(marker)
                .then_some((*marker, apply, inner, close))
        });
        let Some((marker, apply, inner, close)) = found else {
            at += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let mut style = base;
        apply(&mut style);
        let end = close + marker.len();
        paint(styles, at..inner, faint_if(style, true));
        paint(styles, close..end, faint_if(style, true));
        if marker == "`" {
            paint(styles, inner..close, style);
        } else {
            inline(line, inner..close, style, styles);
        }
        at = end;
    }
}

/// Where the marker opened at `open` closes before `limit`, if it opens
/// anything: there is text between the two, and an underscore pair is not
/// inside a word.
fn closing(line: &str, marker: &str, open: usize, limit: usize) -> Option<usize> {
    if !line[open..limit].starts_with(marker) {
        return None;
    }
    let inner = open + marker.len();
    let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    let first = line[inner..limit].chars().next()?;
    if first.is_whitespace() && marker != "`" {
        return None;
    }
    let underscore = marker.starts_with('_');
    if underscore && word(line[..open].chars().next_back()) {
        return None;
    }
    let mut from = inner + first.len_utf8();
    while let Some(found) = line[from..limit].find(marker) {
        let close = from + found;
        let end = close + marker.len();
        // A single star that is half of a pair closes nothing.
        let half =
            marker == "*" && (line[end..limit].starts_with('*') || line[..close].ends_with('*'));
        let spaced = line[..close].ends_with(char::is_whitespace) && marker != "`";
        let in_word = underscore && word(line[end..].chars().next());
        if !half && !spaced && !in_word {
            return Some(close);
        }
        from = end;
    }
    None
}

/// Styles a link, `[text](target)`, opening at `at`. Returns where it ends.
fn link(
    line: &str,
    at: usize,
    limit: usize,
    base: SourceStyle,
    styles: &mut [SourceStyle],
) -> Option<usize> {
    if !line[at..limit].starts_with('[') {
        return None;
    }
    let text_end = at + line[at..limit].find("](")?;
    let target_end = text_end + line[text_end..limit].find(')')?;
    if line[at + 1..text_end].contains('[') || text_end == at + 1 {
        return None;
    }
    let faint = faint_if(base, true);
    paint(styles, at..at + 1, faint);
    let text = SourceStyle { link: true, ..base };
    inline(line, at + 1..text_end, text, styles);
    paint(styles, text_end..target_end + 1, faint);
    Some(target_end + 1)
}

/// The per-byte styles as spans, leaving plain text out.
fn finish(heading: u8, styles: &[SourceStyle]) -> SourceLine {
    let mut spans: Vec<SourceSpan> = Vec::new();
    for (at, style) in styles.iter().enumerate() {
        if *style == SourceStyle::default() {
            continue;
        }
        match spans.last_mut() {
            Some(last) if last.range.end == at && last.style == *style => last.range.end = at + 1,
            _ => spans.push(SourceSpan {
                range: at..at + 1,
                style: *style,
            }),
        }
    }
    SourceLine { heading, spans }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The styled stretches of a one-line source, as `text:flags`. The flags
    /// are the first letters of the style's fields, faint as `.`.
    fn spans(line: &str) -> Vec<String> {
        let styled = style_lines(line).remove(0);
        styled
            .spans
            .iter()
            .map(|span| {
                let style = span.style;
                let flags: String = [
                    (style.strong, 's'),
                    (style.emphasis, 'e'),
                    (style.code, 'c'),
                    (style.strike, 'x'),
                    (style.link, 'l'),
                    (style.faint, '.'),
                ]
                .iter()
                .filter_map(|(on, flag)| on.then_some(*flag))
                .collect();
                format!("{}:{flags}", &line[span.range.clone()])
            })
            .collect()
    }

    #[test]
    fn a_heading_is_heavy_with_a_faint_mark() {
        let lines = style_lines("## Plan\n####### seven\n#tag");
        assert_eq!(
            lines.iter().map(|line| line.heading).collect::<Vec<_>>(),
            [2, 0, 0]
        );
        assert_eq!(spans("## Plan"), ["##:s.", "Plan:s"]);
        assert_eq!(spans("#tag"), Vec::<String>::new());
    }

    #[test]
    fn emphasis_strong_code_and_strike_keep_their_markers_faint() {
        assert_eq!(
            spans("a **b** *c* `d` ~~e~~"),
            [
                "**:s.", "b:s", "**:s.", "*:e.", "c:e", "*:e.", "`:c.", "d:c", "`:c.", "~~:x.",
                "e:x", "~~:x."
            ]
        );
        assert_eq!(
            spans("**a *b* c**"),
            ["**:s.", "a :s", "*:se.", "b:se", "*:se.", " c:s", "**:s."]
        );

        {
            for line in ["2 * 3 * 4", "**open", "snake_case_name", "a ` b"] {
                assert_eq!(spans(line), Vec::<String>::new(), "{line}");
            }
            assert_eq!(spans("`**x**`"), ["`:c.", "**x**:c", "`:c."]);
        }

        {
            assert_eq!(
                spans("**日本** と *語*"),
                ["**:s.", "日本:s", "**:s.", "*:e.", "語:e", "*:e."]
            );
        }
    }

    #[test]
    fn list_markers_task_boxes_and_quote_marks_are_faint() {
        assert_eq!(spans("- one"), ["-:."]);
        assert_eq!(spans("  12. two"), ["12.:."]);
        assert_eq!(spans("* [x] done"), ["*:.", "[x]:."]);
        assert_eq!(
            spans("> > quoted *it*"),
            [">:.", ">:.", "*:e.", "it:e", "*:e."]
        );
        assert_eq!(spans("-not a list"), Vec::<String>::new());

        {
            assert_eq!(spans("---"), ["---:."]);
            assert_eq!(spans("* * *"), ["* * *:."]);
            assert_eq!(
                spans("see [the **docs**](https://x.y)"),
                [
                    "[:.",
                    "the :l",
                    "**:sl.",
                    "docs:sl",
                    "**:sl.",
                    "](https://x.y):."
                ]
            );
        }
    }

    #[test]
    fn a_fence_makes_code_of_every_line_until_it_closes() {
        let lines = style_lines("```rust\n# not a heading\n**x**\n```\n# heading");
        let all = |line: &SourceLine, faint: bool| {
            line.spans.len() == 1 && line.spans[0].style.code && line.spans[0].style.faint == faint
        };
        assert!(all(&lines[0], true) && all(&lines[3], true));
        assert!(all(&lines[1], false) && all(&lines[2], false));
        assert_eq!((lines[1].heading, lines[4].heading), (0, 1));
    }
}
