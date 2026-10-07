//! Turns the items of a text batch into glyphon areas: where each shaped
//! buffer goes, and the straight lines that go with it.

use std::collections::HashMap;
use std::ops::Range;

use glyphon::{
    Buffer, Color as GlyphColor, ContentType, CustomGlyph, RasterizeCustomGlyphRequest,
    RasterizedCustomGlyph, TextBounds,
};
use specular_doc::EntityId;
use specular_scene::{Color, ColumnDraw, Point, Rect, Size, TextRun};

use super::column::{self, rule_rect};
use super::text_layout::{same_shaping, shaping_hash, text_rect};
use super::text_shape::Line;

/// The text of one item.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TextItem<'a> {
    Run(&'a TextRun),
    Column(&'a ColumnDraw),
}

/// One item of a text batch, resolved for drawing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TextDraw<'a> {
    pub(crate) text: TextItem<'a>,
    /// The item's clip in logical pixels, if it has one.
    pub(crate) clip: Option<Rect>,
    pub(crate) opacity: f32,
}

/// A shaped run, kept between frames.
pub(super) struct Shaped {
    pub(super) run: TextRun,
    pub(super) buffer: Buffer,
    pub(super) size: Size,
    pub(super) lines: Vec<Line>,
    pub(super) last_used: u64,
}

/// One glyphon text area, before its buffer and lines are borrowed.
pub(super) struct Area<'a> {
    /// `None` is an area of lines only.
    pub(super) buffer: Option<&'a Buffer>,
    /// Top-left in the item's space.
    pub(super) origin: Point,
    pub(super) color: GlyphColor,
    /// Its lines, as a range of the batch's custom glyphs.
    pub(super) lines: Range<usize>,
    pub(super) bounds: TextBounds,
}

/// Collects the areas and lines of one batch.
pub(super) struct Areas<'a> {
    shaped: &'a HashMap<u64, Shaped>,
    placed: Vec<Area<'a>>,
    lines: Vec<CustomGlyph>,
    hairline: f32,
    whole: TextBounds,
    to_layout: f32,
    /// How tall each owned column's rows came out.
    heights: Vec<(EntityId, f32)>,
}

impl<'a> Areas<'a> {
    /// `hairline` is the thinnest a line may be, in the item's units.
    /// `whole` is the bounds of an unclipped item, and `to_layout` takes
    /// logical pixels to the physical pixels glyphon lays out in.
    pub(super) fn new(
        shaped: &'a HashMap<u64, Shaped>,
        hairline: f32,
        whole: TextBounds,
        to_layout: f32,
    ) -> Self {
        Self {
            shaped,
            placed: Vec::new(),
            lines: Vec::new(),
            hairline,
            whole,
            to_layout,
            heights: Vec::new(),
        }
    }

    /// The areas in paint order, the lines their ranges index, and the
    /// height of each column that has an owner.
    pub(super) fn finish(self) -> (Vec<Area<'a>>, Vec<CustomGlyph>, Vec<(EntityId, f32)>) {
        (self.placed, self.lines, self.heights)
    }

    fn shaped(&self, run: &TextRun) -> Option<&'a Shaped> {
        (self.shaped.get(&shaping_hash(run))).filter(|shaped| same_shaping(&shaped.run, run))
    }

    fn bounds(&self, clip: Option<Rect>) -> TextBounds {
        let edge = |logical: f32| (logical * self.to_layout).round() as i32;
        clip.map_or(self.whole, |clip| TextBounds {
            left: edge(clip.x),
            top: edge(clip.y),
            right: edge(clip.right()),
            bottom: edge(clip.bottom()),
        })
    }

    fn line(&mut self, rect: Rect, color: Color, opacity: f32) {
        self.lines.push(CustomGlyph {
            id: 0,
            left: rect.x,
            top: rect.y,
            width: rect.width.max(self.hairline),
            height: rect.height.max(self.hairline),
            color: Some(glyph_color(color, opacity)),
            snap_to_physical_pixel: true,
            metadata: 0,
        });
    }

    /// One run with its top-left moved by `offset`, and its underlines and
    /// strikes.
    pub(super) fn run(&mut self, run: &TextRun, offset: Point, draw: &TextDraw<'_>) {
        let Some(shaped) = self.shaped(run) else {
            return;
        };
        let rect = text_rect(run, shaped.size);
        let start = self.lines.len();
        for line in &shaped.lines {
            self.line(line.rect, line.color.unwrap_or(run.color), draw.opacity);
        }
        self.placed.push(Area {
            buffer: Some(&shaped.buffer),
            origin: Point::new(rect.x + offset.x, rect.y + offset.y),
            color: glyph_color(run.color, draw.opacity),
            lines: start..self.lines.len(),
            bounds: self.bounds(draw.clip),
        });
    }

    /// A column: one area for the rules of every row, then each cell. Rows
    /// outside `visible`, a rect in the item's space, are left out.
    pub(super) fn column(
        &mut self,
        column: &ColumnDraw,
        visible: Option<Rect>,
        draw: &TextDraw<'_>,
    ) {
        let layout = column::layout(column, |run| {
            self.shaped(run)
                .map_or(Size::default(), |shaped| shaped.size)
        });
        if let Some(owner) = &column.owner {
            self.heights.push((owner.clone(), layout.height()));
        }
        let origin = layout.bounds(column).origin();
        let start = self.lines.len();
        let mut cells = Vec::new();
        for (row, at) in layout.rows(column) {
            let rect = layout.row_rect(column, at);
            // The gap above belongs to the row: a rule may reach into it.
            let reach = Rect::new(rect.x, rect.y - row.gap, rect.width, rect.height + row.gap);
            if visible.is_some_and(|visible| !visible.intersects(reach)) {
                continue;
            }
            for rule in &row.rules {
                let within = rule_rect(rule, at);
                let placed = Rect::new(
                    within.x,
                    rect.y - origin.y + within.y,
                    within.width,
                    within.height,
                );
                self.line(placed, rule.color, draw.opacity);
            }
            cells.extend(row.cells.iter().map(|cell| (cell, rect.origin())));
        }
        if self.lines.len() > start {
            self.placed.push(Area {
                buffer: None,
                origin,
                color: GlyphColor::rgba(0, 0, 0, 0),
                lines: start..self.lines.len(),
                bounds: self.bounds(draw.clip),
            });
        }
        for (cell, offset) in cells {
            self.run(cell, offset, draw);
        }
    }
}

fn glyph_color(color: Color, opacity: f32) -> GlyphColor {
    let alpha = (f32::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8;
    GlyphColor::rgba(color.r, color.g, color.b, alpha)
}

/// A line is a solid block of coverage the size it was asked for.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the signature is glyphon's rasteriser callback"
)]
pub(super) fn solid(request: RasterizeCustomGlyphRequest) -> Option<RasterizedCustomGlyph> {
    Some(RasterizedCustomGlyph {
        data: vec![u8::MAX; usize::from(request.width) * usize::from(request.height)],
        content_type: ContentType::Mask,
    })
}
