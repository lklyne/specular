use glam::Vec2;
use specular_scene::{Item, Space};

use super::super::place::tests::{placed, rect, text, view};
use super::*;

/// Each batch as its pipeline and the scene items it draws, in order.
fn batches(items: Vec<Item>) -> Vec<(Prim, Vec<usize>)> {
    let view = view(Vec2::ZERO, 1.0);
    let (placed, _) = placed(&view, items);
    batch(&placed, &view)
        .into_iter()
        .map(|batch| {
            let items = batch.members.iter().map(|&at| placed[at].item).collect();
            (batch.prim, items)
        })
        .collect()
}

const CANVAS_TEXT: Prim = Prim::Text(Space::Canvas);

/// A sticky note at `x`: its rect, then its text inside it.
fn note(x: f32) -> [Item; 2] {
    [
        Item::canvas(rect(x, 0.0, 80.0, 80.0)),
        Item::canvas(text(x + 10.0, 10.0, 14.0)),
    ]
}

#[test]
fn separate_notes_share_one_shape_batch_and_one_text_batch() {
    let items = [note(0.0), note(100.0), note(200.0)].concat();
    assert_eq!(
        batches(items),
        [(Prim::Shape, vec![0, 2, 4]), (CANVAS_TEXT, vec![1, 3, 5])]
    );
}
