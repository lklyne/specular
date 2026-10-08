//! Groups placed items into draw batches without changing what is on top.
//!
//! Pages and every other item share one z-order, and each pipeline wants its
//! items in as few draws as possible: glyphon draws everything one
//! `TextRenderer` prepared in a single call, shapes are one instanced draw,
//! paths one indexed draw. Painting strictly in list order would make a
//! batch per item. Instead an item joins the earliest batch of its own kind
//! that comes at or after the last batch it overlaps. Items that do not
//! overlap have no visible order, so five hundred separate sticky notes are
//! one shape batch and one text batch, while a note dropped on a page still
//! paints after that page. A page is always a batch of its own, so the list
//! breaks at each page that something overlaps.
//!
//! "The last batch it overlaps" is asked of a grid over the viewport that
//! holds every item placed so far, so an item is tested against its
//! neighbours and not against everything before it.

use std::collections::HashMap;

use specular_scene::Rect;

use super::place::{Placed, Prim, Scissor, ViewTransform};

/// Grid cells across and down the viewport.
const COLUMNS: usize = 32;
const ROWS: usize = 20;

/// One draw: items of one pipeline under one scissor, in paint order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Batch {
    pub(crate) prim: Prim,
    /// Scissor for the draw; `None` is the whole target. Always `None` for
    /// text, which is clipped per run instead.
    pub(crate) scissor: Option<Scissor>,
    /// Indices into the placed list, in paint order.
    pub(crate) members: Vec<usize>,
}

/// What may share a draw: a pipeline and a scissor.
type Kind = (Prim, Option<Scissor>);

/// Whether items of `prim` may share a batch at all. Each page and image
/// binds its own texture, so those never do.
fn shares(prim: Prim) -> bool {
    !matches!(prim, Prim::Page | Prim::Image)
}

/// An item already placed in a batch, as the grid holds it.
#[derive(Debug, Clone, Copy)]
struct Entry {
    bounds: Rect,
    batch: u32,
}

/// Batches a frame's items. Kept between frames for its allocations.
#[derive(Debug, Default)]
pub(crate) struct Batcher {
    /// `COLUMNS * ROWS` cells over the viewport, each with the items that
    /// touch it.
    cells: Vec<Vec<Entry>>,
    /// The batches of each kind, in order.
    of_kind: HashMap<Kind, Vec<u32>>,
    /// Bounds compared by the last [`batch`](Self::batch).
    overlap_tests: u64,
}

impl Batcher {
    /// How many pairs of bounds the last frame compared.
    #[cfg(test)]
    pub(crate) fn overlap_tests(&self) -> u64 {
        self.overlap_tests
    }

    /// Batches `placed`, in the order the batches are drawn.
    pub(crate) fn batch(&mut self, placed: &[Placed], view: &ViewTransform) -> Vec<Batch> {
        self.cells.resize_with(COLUMNS * ROWS, Vec::new);
        self.cells.iter_mut().for_each(Vec::clear);
        self.of_kind.clear();
        self.overlap_tests = 0;
        let cell = Rect::new(
            0.0,
            0.0,
            (view.viewport.x / COLUMNS as f32).max(1.0),
            (view.viewport.y / ROWS as f32).max(1.0),
        );
        let mut batches: Vec<Batch> = Vec::new();
        for (index, item) in placed.iter().enumerate() {
            let scissor = match (item.prim, item.clip) {
                (Prim::Text(_), _) | (_, None) => None,
                (_, Some(clip)) => match view.scissor(clip) {
                    Some(scissor) => Some(scissor),
                    None => continue,
                },
            };
            let kind = (item.prim, scissor);
            let cells = cells_of(item.bounds, cell);
            // The last batch needs no overlap test: joining it changes no
            // order.
            let joins_last = (batches.last())
                .is_some_and(|last| shares(item.prim) && (last.prim, last.scissor) == kind);
            let target = if joins_last {
                Some(batches.len() - 1)
            } else if shares(item.prim) {
                let floor = self.last_overlapped(item.bounds, &cells);
                // The earliest batch of this kind at or after `floor`.
                self.of_kind.get(&kind).and_then(|of_kind| {
                    let at = of_kind.partition_point(|&batch| batch < floor);
                    of_kind.get(at).map(|&batch| batch as usize)
                })
            } else {
                None
            };
            let batch = if let Some(at) = target {
                batches[at].members.push(index);
                at
            } else {
                batches.push(Batch {
                    prim: item.prim,
                    scissor,
                    members: vec![index],
                });
                let at = batches.len() - 1;
                self.of_kind.entry(kind).or_default().push(at as u32);
                at
            };
            let entry = Entry {
                bounds: item.bounds,
                batch: batch as u32,
            };
            for (column, row) in cells.iter() {
                self.cells[row * COLUMNS + column].push(entry);
            }
        }
        batches
    }

    /// The last batch holding an item that `bounds` overlaps, or the first
    /// batch when it overlaps nothing.
    fn last_overlapped(&mut self, bounds: Rect, cells: &CellRange) -> u32 {
        let mut last = 0;
        for (column, row) in cells.iter() {
            for entry in &self.cells[row * COLUMNS + column] {
                // Later batches are the only ones that can raise the answer.
                if entry.batch > last {
                    self.overlap_tests += 1;
                    if entry.bounds.intersects(bounds) {
                        last = entry.batch;
                    }
                }
            }
        }
        last
    }
}

/// The cells a rect touches: columns and rows, both ends included.
#[derive(Debug, Clone, Copy)]
struct CellRange {
    columns: (usize, usize),
    rows: (usize, usize),
}

impl CellRange {
    fn iter(&self) -> impl Iterator<Item = (usize, usize)> + use<> {
        let (columns, rows) = (self.columns, self.rows);
        (rows.0..=rows.1)
            .flat_map(move |row| (columns.0..=columns.1).map(move |column| (column, row)))
    }
}

/// The cells `bounds` touches. Bounds are inside the viewport, but a rect
/// that is not is put in the nearest cells, which keeps the answer right.
fn cells_of(bounds: Rect, cell: Rect) -> CellRange {
    let index = |at: f32, size: f32, count: usize| ((at / size).max(0.0) as usize).min(count - 1);
    CellRange {
        columns: (
            index(bounds.x, cell.width, COLUMNS),
            index(bounds.right(), cell.width, COLUMNS),
        ),
        rows: (
            index(bounds.y, cell.height, ROWS),
            index(bounds.bottom(), cell.height, ROWS),
        ),
    }
}

/// Batches `placed` with a batcher of its own.
#[cfg(test)]
pub(crate) fn batch(placed: &[Placed], view: &ViewTransform) -> Vec<Batch> {
    Batcher::default().batch(placed, view)
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
