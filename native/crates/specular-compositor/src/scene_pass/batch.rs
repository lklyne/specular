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

use specular_scene::Rect;

use super::place::{Placed, Prim, Scissor, ViewTransform};

/// One draw: items of one pipeline under one scissor, in paint order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Batch {
    pub(crate) prim: Prim,
    /// Scissor for the draw; `None` is the whole target. Always `None` for
    /// text, which is clipped per run instead.
    pub(crate) scissor: Option<Scissor>,
    /// Indices into the placed list, in paint order.
    pub(crate) members: Vec<usize>,
    /// Union of the members' bounds, for a quick miss.
    bounds: Rect,
}

impl Batch {
    fn overlaps(&self, bounds: Rect, placed: &[Placed]) -> bool {
        self.bounds.intersects(bounds)
            && self
                .members
                .iter()
                .rev()
                .any(|&member| placed[member].bounds.intersects(bounds))
    }

    /// Whether an item of `prim` under `scissor` may join. Each page and
    /// image binds its own texture, so those never share a batch.
    fn accepts(&self, prim: Prim, scissor: Option<Scissor>) -> bool {
        self.prim == prim && self.scissor == scissor && !matches!(prim, Prim::Page | Prim::Image)
    }
}

/// Batches `placed`, in the order the batches are drawn.
pub(crate) fn batch(placed: &[Placed], view: &ViewTransform) -> Vec<Batch> {
    let mut batches: Vec<Batch> = Vec::new();
    for (index, item) in placed.iter().enumerate() {
        let scissor = match (item.prim, item.clip) {
            (Prim::Text(_), _) | (_, None) => None,
            (_, Some(clip)) => match view.scissor(clip) {
                Some(scissor) => Some(scissor),
                None => continue,
            },
        };
        // The last batch needs no overlap test: joining it changes no order.
        let joins_last = batches
            .last()
            .is_some_and(|last| last.accepts(item.prim, scissor));
        let target = if joins_last {
            Some(batches.len() - 1)
        } else {
            let floor = batches
                .iter()
                .rposition(|batch| batch.overlaps(item.bounds, placed))
                .unwrap_or(0);
            (floor..batches.len()).find(|&at| batches[at].accepts(item.prim, scissor))
        };
        match target {
            Some(at) => {
                let batch = &mut batches[at];
                batch.members.push(index);
                batch.bounds = batch.bounds.union(item.bounds);
            }
            None => batches.push(Batch {
                prim: item.prim,
                scissor,
                members: vec![index],
                bounds: item.bounds,
            }),
        }
    }
    batches
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
