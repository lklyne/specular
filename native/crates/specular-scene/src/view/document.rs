//! Documents: a markdown file read as rows of text inside a card, and its
//! source as rows of styled text while it is edited.
//!
//! The two views do not share a layout. Read, the markers are gone, a list
//! item is a marker cell and a text cell and a table is a grid, so nothing
//! in it maps back to a byte of the file. Edited, every source line is one
//! row holding exactly its own characters, which is what lets the caret and
//! the selection be measured. They share the renderer's rows, the sizes and
//! the colours.
//!
//! The sizes and colours are Electron's read-only note: 14 on a 1.5 line,
//! headings at 1.4, 1.2 and 1.1 times that in weight 600, 12 units of
//! padding that scroll with the text. Each block is a [`Row`]; the renderer
//! stacks them, because only it knows how tall wrapped text comes out.

use specular_doc::Entity;
use specular_interact::{NOTE_PADDING, NoteState, source_rows};

use super::frame::{Frame, canvas_rect};
use super::{editing, palette};
use crate::markdown::{self, Block, BlockKind, ColumnAlign, Inline, InlineSpan, Marker, Table};
use crate::{
    Color, ColumnDraw, FontFamily, Item, Point, RectDraw, Row, RowRule, RuleHeight, Scene,
    SpanStyle, TextAlign, TextRun, TextSpan,
};

const CORNER_RADIUS: f32 = 4.0;
const SIZE: f32 = 14.0;
/// Line height as a multiple of the font size, headings included.
const LINE: f32 = 1.5;
/// Sizes of the first three heading levels as multiples of [`SIZE`]. Deeper
/// headings are body size.
const HEADING_SCALE: [f32; 3] = [1.4, 1.2, 1.1];
/// Weight of headings, strong text and a table's head.
const HEAVY: u16 = TextRun::HEAVY;
/// Space between blocks: one blank line, as the source has between them.
const BLOCK_GAP: f32 = SIZE * LINE;
/// How far each list level moves its text right. The marker hangs in it.
const LIST_INDENT: f32 = 24.0;
/// Space between a marker's right edge and its item's text.
const MARKER_GAP: f32 = 6.0;
const QUOTE_BAR: f32 = 2.0;
/// How far each quote level moves its text right: the bar and 0.6 em.
const QUOTE_INDENT: f32 = QUOTE_BAR + 0.6 * SIZE;
/// Room around a table cell's text.
const CELL_PADDING: Point = Point::new(6.0, 4.0);
const LINE_WIDTH: f32 = 1.0;

const INK: Color = palette::INK;
const LINK: Color = Color::rgb(0x25, 0x63, 0xeb);
/// Quoted text and its bar: the ink at 85%.
const QUOTE_INK: Color = INK.with_alpha(217);
/// Bullets, tick boxes and image stand-ins: the ink at 60%.
const FAINT_INK: Color = INK.with_alpha(153);
/// A status line in place of the text: the ink at 40%.
const STATUS_INK: Color = INK.with_alpha(102);
/// Dividers and the line under a table's head: the ink at 25%.
const RULE: Color = INK.with_alpha(64);
/// The line under a table's body rows.
const FAINT_RULE: Color = INK.with_alpha(31);

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, note: &NoteState, scene: &mut Scene) {
    let rect = canvas_rect(entity.rect);
    scene.push(palette::card_shadow(rect, CORNER_RADIUS));
    scene.push(Item::canvas(
        RectDraw::filled(rect, palette::CARD).with_corner_radius(CORNER_RADIUS),
    ));
    if let Some(source) = frame.app.editing_text(&entity.id) {
        edited(frame, entity, source, scene);
        return;
    }
    let inner = rect.outset(-NOTE_PADDING);
    let status = match note {
        NoteState::Ready(text) => {
            let rows = rows(&markdown::parse(text), inner.width.max(0.0));
            if !rows.is_empty() {
                let column = ColumnDraw {
                    origin: inner.origin(),
                    width: inner.width.max(0.0),
                    height: inner.height.max(0.0),
                    scroll: frame.app.note_scroll(&entity.id),
                    rows,
                    owner: Some(entity.id.clone()),
                };
                scene.push(Item::canvas(column).clipped(rect));
            }
            return;
        }
        NoteState::Loading => "Loading…",
        NoteState::Missing => "File not found",
        NoteState::Failed => "Could not read file",
    };
    let run = TextRun {
        wrap_width: Some(inner.width.max(0.0)),
        line_height: SIZE * LINE,
        ..TextRun::new(status, inner.origin(), SIZE, STATUS_INK)
    };
    scene.push(Item::canvas(run).clipped(rect));
}

/// The Document's source while it is edited: a row for each line, the
/// selection behind them and the caret over them. The editor has the scroll
/// in the frame already, so the column is placed and not scrolled again.
fn edited(frame: &Frame<'_>, entity: &Entity, source: &str, scene: &mut Scene) {
    let rect = canvas_rect(entity.rect);
    let id = &entity.id;
    let Some(text_frame) = frame.app.text_frame(id) else {
        return;
    };
    editing::selection(frame, id, Some(rect), scene);
    let rows = source_rows(source, &text_frame.spec)
        .into_iter()
        .map(|row| {
            let text = &source[row.range.clone()];
            let cell = TextRun::source(
                text,
                &row.spec,
                &row.spans,
                Point::default(),
                [INK, FAINT_INK, LINK],
            );
            Row {
                min_height: row.spec.line_height,
                cells: if text.is_empty() {
                    Vec::new()
                } else {
                    vec![cell]
                },
                ..Row::default()
            }
        })
        .collect();
    let column = ColumnDraw {
        origin: Point::new(text_frame.origin.x as f32, text_frame.origin.y as f32),
        width: text_frame.spec.wrap_width.unwrap_or(0.0),
        height: (rect.height - NOTE_PADDING * 2.0).max(0.0),
        scroll: 0.0,
        rows,
        owner: None,
    };
    scene.push(Item::canvas(column).clipped(rect));
    editing::caret(frame, id, Some(rect), INK, scene);
}

/// The rows of a document `width` units wide.
fn rows(blocks: &[Block], width: f32) -> Vec<Row> {
    let mut rows = Vec::with_capacity(blocks.len());
    let mut above: Option<&Block> = None;
    for block in blocks {
        let place = Place::new(block, above, width);
        match &block.kind {
            BlockKind::Heading { level, text } => {
                let scale =
                    (HEADING_SCALE.get(usize::from(*level).saturating_sub(1))).unwrap_or(&1.0);
                let mut run = place.run(text, SIZE * scale);
                run.weight = HEAVY;
                rows.push(place.row(vec![run]));
            }
            BlockKind::Paragraph(text) => rows.push(place.row(vec![place.run(text, SIZE)])),
            BlockKind::Item { marker, text } => {
                let mut cells = vec![place.marker(*marker)];
                if !text.text.is_empty() {
                    cells.push(place.run(text, SIZE));
                }
                rows.push(place.row(cells));
            }
            BlockKind::Code(code) => {
                let run = TextRun {
                    family: FontFamily::Monospace,
                    ..place.plain(code.clone(), SIZE)
                };
                rows.push(place.row(vec![run]));
            }
            BlockKind::Rule => {
                let mut row = place.row(Vec::new());
                row.min_height = SIZE * LINE;
                row.rules
                    .push(place.line(RuleHeight::Middle(LINE_WIDTH), RULE));
                rows.push(row);
            }
            BlockKind::Table(table) => place.table(table, &mut rows),
        }
        above = Some(block);
    }
    rows
}

/// Where one block's rows go: its gap, its left edge, and the quote bars
/// beside it.
struct Place {
    gap: f32,
    /// Left edge of the block's text, from the column's left.
    left: f32,
    /// Width the text wraps in.
    width: f32,
    quoted: bool,
    bars: Vec<RowRule>,
}

impl Place {
    fn new(block: &Block, above: Option<&Block>, width: f32) -> Self {
        let gap = match above {
            None => 0.0,
            Some(_) if block.tight => 0.0,
            Some(_) => BLOCK_GAP,
        };
        let left = f32::from(block.quote) * QUOTE_INDENT + f32::from(block.indent) * LIST_INDENT;
        let bars = (0..block.quote)
            .map(|level| RowRule {
                x: f32::from(level) * QUOTE_INDENT,
                width: QUOTE_BAR,
                // A bar runs through the gap when the block above is in the
                // same quote, so one quote has one bar.
                height: if above.is_some_and(|above| above.quote > level) {
                    RuleHeight::RowAndGap
                } else {
                    RuleHeight::Row
                },
                color: QUOTE_INK,
            })
            .collect();
        Self {
            gap,
            left,
            width: (width - left).max(0.0),
            quoted: block.quote > 0,
            bars,
        }
    }

    fn ink(&self) -> Color {
        if self.quoted { QUOTE_INK } else { INK }
    }

    fn row(&self, cells: Vec<TextRun>) -> Row {
        Row {
            gap: self.gap,
            cells,
            rules: self.bars.clone(),
            ..Row::default()
        }
    }

    /// A line across the block's width.
    fn line(&self, height: RuleHeight, color: Color) -> RowRule {
        RowRule {
            x: self.left,
            width: self.width,
            height,
            color,
        }
    }

    /// Unstyled text filling the block's width.
    fn plain(&self, text: String, size: f32) -> TextRun {
        TextRun {
            wrap_width: Some(self.width),
            line_height: size * LINE,
            ..TextRun::new(text, Point::new(self.left, 0.0), size, self.ink())
        }
    }

    /// Inline content filling the block's width. Quoted text is italic.
    fn run(&self, inline: &Inline, size: f32) -> TextRun {
        TextRun {
            italic: self.quoted,
            spans: inline.spans.iter().map(span).collect(),
            ..self.plain(inline.text.clone(), size)
        }
    }

    /// An item's marker, ending just left of the item's text.
    fn marker(&self, marker: Marker) -> TextRun {
        let (text, color) = match marker {
            Marker::Bullet => ("•".to_owned(), FAINT_INK),
            Marker::Number(number) => (format!("{number}."), self.ink()),
            Marker::Task { done: false } => ("☐".to_owned(), FAINT_INK),
            Marker::Task { done: true } => ("☑".to_owned(), FAINT_INK),
        };
        TextRun {
            align: TextAlign::Right,
            line_height: SIZE * LINE,
            ..TextRun::new(text, Point::new(self.left - MARKER_GAP, 0.0), SIZE, color)
        }
    }

    /// A table: a row for the head and one for each body row, in columns of
    /// equal width with a line under each.
    fn table(&self, table: &Table, rows: &mut Vec<Row>) {
        let column_width = self.width / table.columns.len().max(1) as f32;
        let mut push = |cells: &[Inline], head: bool| {
            let cells = (cells.iter().zip(&table.columns).enumerate())
                .filter(|(_, (cell, _))| !cell.text.is_empty())
                .map(|(at, (cell, align))| TextRun {
                    origin: Point::new(
                        self.left + at as f32 * column_width + CELL_PADDING.x,
                        CELL_PADDING.y,
                    ),
                    wrap_width: Some((column_width - CELL_PADDING.x * 2.0).max(0.0)),
                    weight: if head { HEAVY } else { 400 },
                    align: match align {
                        ColumnAlign::Left => TextAlign::Left,
                        ColumnAlign::Centre => TextAlign::Centre,
                        ColumnAlign::Right => TextAlign::Right,
                    },
                    ..self.run(cell, SIZE)
                })
                .collect();
            let mut row = self.row(cells);
            // The head takes the block's gap; the body rows sit under it.
            if !head {
                row.gap = 0.0;
            }
            row.min_height = SIZE * LINE + CELL_PADDING.y * 2.0;
            row.bottom_padding = CELL_PADDING.y;
            let color = if head { RULE } else { FAINT_RULE };
            row.rules
                .push(self.line(RuleHeight::Bottom(LINE_WIDTH), color));
            rows.push(row);
        };
        push(&table.head, true);
        for cells in &table.rows {
            push(cells, false);
        }
    }
}

fn span(span: &InlineSpan) -> TextSpan {
    let style = span.style;
    TextSpan {
        range: span.range.clone(),
        style: SpanStyle {
            family: style.code.then_some(FontFamily::Monospace),
            weight: style.strong.then_some(HEAVY),
            italic: style.emphasis.then_some(true),
            color: if style.link {
                Some(LINK)
            } else if style.image {
                Some(FAINT_INK)
            } else {
                None
            },
            underline: style.link,
            strike: style.strike,
        },
    }
}
