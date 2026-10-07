//! The pulldown-cmark event stream folded into [`Block`]s.

use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::{Block, BlockKind, ColumnAlign, Inline, InlineSpan, InlineStyle, Marker, Table};

/// The blocks of `markdown`, top to bottom.
pub(crate) fn parse(markdown: &str) -> Vec<Block> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    let mut builder = Builder::default();
    for event in Parser::new_ext(markdown, options) {
        builder.event(event);
    }
    builder.close_text();
    builder.blocks
}

/// What the inline content being gathered will become.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Holder {
    Heading(u8),
    Paragraph,
    /// A list item's first paragraph. `tight` when the list has no blank
    /// lines, which the parser shows by leaving the paragraph tags out.
    Item {
        marker: Marker,
        tight: bool,
    },
    Cell,
}

#[derive(Debug, Default)]
struct Builder {
    blocks: Vec<Block>,
    quote: u8,
    /// The next number of each open list; `None` for an unordered one.
    lists: Vec<Option<u64>>,
    /// How many blocks there were when the outermost open list began.
    list_start: usize,
    /// The marker of an item whose first paragraph has not started.
    marker: Option<Marker>,
    /// The inline content being gathered, and what it is for.
    text: Option<(Holder, Inline)>,
    style: InlineStyle,
    /// Where each open image's stand-in text begins.
    images: Vec<usize>,
    code: Option<String>,
    table: Option<Table>,
    in_head: bool,
}

impl Builder {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => match &mut self.code {
                Some(code) => code.push_str(&text),
                None => self.push_text(&text),
            },
            Event::Code(text) => self.styled(|style| style.code = true, &text),
            Event::Html(text) | Event::InlineHtml(text) => self.push_text(&text),
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.push_text("\n"),
            Event::Rule => {
                self.close_text();
                self.push(BlockKind::Rule, false);
            }
            Event::TaskListMarker(done) => {
                let task = Marker::Task { done };
                match &mut self.text {
                    Some((Holder::Item { marker, .. }, _)) => *marker = task,
                    Some(_) | None => self.marker = Some(task),
                }
            }
            // Maths and footnotes are not turned on.
            Event::InlineMath(_) | Event::DisplayMath(_) | Event::FootnoteReference(_) => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                let holder = match self.marker.take() {
                    Some(marker) => Holder::Item {
                        marker,
                        tight: false,
                    },
                    None => Holder::Paragraph,
                };
                self.open(holder);
            }
            Tag::Heading { level, .. } => self.open(Holder::Heading(heading_level(level))),
            Tag::BlockQuote(_) => {
                self.close_text();
                self.quote = self.quote.saturating_add(1);
            }
            // A fence's info string names a language; nothing highlights yet.
            Tag::CodeBlock(_) => {
                self.close_text();
                self.code = Some(String::new());
            }
            Tag::List(first) => {
                self.close_text();
                if self.lists.is_empty() {
                    self.list_start = self.blocks.len();
                }
                self.lists.push(first);
            }
            Tag::Item => {
                self.close_text();
                let number = self.lists.last_mut().and_then(Option::as_mut);
                self.marker = Some(match number {
                    Some(number) => {
                        let marker = Marker::Number(*number);
                        *number += 1;
                        marker
                    }
                    None => Marker::Bullet,
                });
            }
            Tag::Table(columns) => {
                self.close_text();
                let columns = columns.into_iter().map(column_align).collect();
                self.table = Some(Table {
                    columns,
                    ..Table::default()
                });
            }
            Tag::TableHead => self.in_head = true,
            Tag::TableRow => {
                if let Some(table) = &mut self.table {
                    table.rows.push(Vec::new());
                }
            }
            Tag::TableCell => self.open(Holder::Cell),
            Tag::Emphasis => self.style.emphasis = true,
            Tag::Strong => self.style.strong = true,
            Tag::Strikethrough => self.style.strike = true,
            Tag::Link { .. } => self.style.link = true,
            Tag::Image { .. } => {
                self.style.image = true;
                self.push_text("[image: ");
                let at = self
                    .text
                    .as_ref()
                    .map_or(0, |(_, inline)| inline.text.len());
                self.images.push(at);
            }
            // Its text arrives as `Html` events and is shown as written.
            Tag::HtmlBlock => self.close_text(),
            // Extensions that are not turned on.
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::HtmlBlock | TagEnd::TableCell => {
                self.close_text();
            }
            TagEnd::BlockQuote(_) => {
                self.close_text();
                self.quote = self.quote.saturating_sub(1);
            }
            TagEnd::CodeBlock => {
                if let Some(mut code) = self.code.take() {
                    if code.ends_with('\n') {
                        code.pop();
                    }
                    self.push(BlockKind::Code(code), false);
                }
            }
            TagEnd::List(_) => {
                self.close_text();
                self.lists.pop();
            }
            TagEnd::Item => {
                self.close_text();
                // An item with nothing in it still shows its marker.
                if let Some(marker) = self.marker.take() {
                    let tight = self.blocks.len() > self.list_start;
                    let text = Inline::default();
                    self.push(BlockKind::Item { marker, text }, tight);
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.push(BlockKind::Table(table), false);
                }
            }
            TagEnd::TableHead => self.in_head = false,
            TagEnd::Emphasis => self.style.emphasis = false,
            TagEnd::Strong => self.style.strong = false,
            TagEnd::Strikethrough => self.style.strike = false,
            TagEnd::Link => self.style.link = false,
            TagEnd::Image => {
                if let (Some(start), Some((_, inline))) = (self.images.pop(), self.text.as_mut())
                    && inline.text.len() == start
                {
                    // No alt text: `[image: ` becomes `[image`.
                    inline.text.truncate(start - ": ".len());
                    if let Some(span) = inline.spans.last_mut() {
                        span.range.end = inline.text.len();
                    }
                }
                self.push_text("]");
                self.style.image = !self.images.is_empty();
            }
            _ => {}
        }
    }

    /// Starts gathering inline content for `holder`.
    fn open(&mut self, holder: Holder) {
        self.close_text();
        self.text = Some((holder, Inline::default()));
    }

    /// Adds `text` in the current style, starting a block for it if none is
    /// open: a tight list item's text and an HTML block's have no tags.
    fn push_text(&mut self, text: &str) {
        if self.text.is_none() {
            let holder = match self.marker.take() {
                Some(marker) => Holder::Item {
                    marker,
                    tight: true,
                },
                None => Holder::Paragraph,
            };
            self.text = Some((holder, Inline::default()));
        }
        let Some((_, inline)) = &mut self.text else {
            return;
        };
        let start = inline.text.len();
        inline.text.push_str(text);
        let end = inline.text.len();
        if self.style == InlineStyle::default() || start == end {
            return;
        }
        match inline.spans.last_mut() {
            Some(last) if last.range.end == start && last.style == self.style => {
                last.range.end = end;
            }
            _ => inline.spans.push(InlineSpan {
                range: start..end,
                style: self.style,
            }),
        }
    }

    /// Adds `text` with one more style on it than the surrounding text.
    fn styled(&mut self, add: impl FnOnce(&mut InlineStyle), text: &str) {
        let outer = self.style;
        add(&mut self.style);
        self.push_text(text);
        self.style = outer;
    }

    /// Finishes the inline content being gathered, if any.
    fn close_text(&mut self) {
        let Some((holder, mut text)) = self.text.take() else {
            return;
        };
        // An HTML block ends in the line break that closed it.
        let trimmed = text.text.trim_end_matches('\n').len();
        text.text.truncate(trimmed);
        text.spans.retain_mut(|span| {
            span.range.end = span.range.end.min(trimmed);
            span.range.start < span.range.end
        });
        match holder {
            Holder::Heading(level) => self.push(BlockKind::Heading { level, text }, false),
            Holder::Paragraph => self.push(BlockKind::Paragraph(text), false),
            Holder::Item { marker, tight } => {
                let tight = tight && self.blocks.len() > self.list_start;
                self.push(BlockKind::Item { marker, text }, tight);
            }
            Holder::Cell => {
                if let Some(table) = &mut self.table {
                    let row = if self.in_head {
                        Some(&mut table.head)
                    } else {
                        table.rows.last_mut()
                    };
                    if let Some(row) = row {
                        row.push(text);
                    }
                }
            }
        }
    }

    fn push(&mut self, kind: BlockKind, tight: bool) {
        self.blocks.push(Block {
            kind,
            quote: self.quote,
            indent: u8::try_from(self.lists.len()).unwrap_or(u8::MAX),
            tight,
        });
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn column_align(alignment: Alignment) -> ColumnAlign {
    match alignment {
        Alignment::None | Alignment::Left => ColumnAlign::Left,
        Alignment::Center => ColumnAlign::Centre,
        Alignment::Right => ColumnAlign::Right,
    }
}
