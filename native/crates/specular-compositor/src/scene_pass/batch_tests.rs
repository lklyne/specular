use glam::Vec2;
use specular_doc::EntityId;
use specular_scene::{Item, PageDraw, Space};

use super::super::place::tests::{placed, rect, text, view};
use super::*;

fn page(x: f32, y: f32) -> PageDraw {
    PageDraw {
        page: EntityId::new("page"),
        rect: Rect::new(x, y, 200.0, 200.0),
        corner_radius: 0.0,
    }
}

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

#[test]
fn a_note_over_another_notes_text_starts_a_new_batch() {
    // The second note sits on the first one's text, so its rect has to
    // paint after that text.
    let items = [note(0.0), note(20.0)].concat();
    assert_eq!(
        batches(items),
        [
            (Prim::Shape, vec![0]),
            (CANVAS_TEXT, vec![1]),
            (Prim::Shape, vec![2]),
            (CANVAS_TEXT, vec![3])
        ]
    );
}

#[test]
fn an_item_over_a_page_is_drawn_after_it_and_one_under_before() {
    let items = vec![
        Item::canvas(rect(50.0, 50.0, 20.0, 20.0)),
        Item::canvas(page(0.0, 0.0)),
        Item::canvas(rect(60.0, 60.0, 20.0, 20.0)),
    ];
    assert_eq!(
        batches(items),
        [
            (Prim::Shape, vec![0]),
            (Prim::Page, vec![1]),
            (Prim::Shape, vec![2])
        ]
    );
}

#[test]
fn chrome_beside_each_page_does_not_break_at_pages_it_misses() {
    // Two pages, each with a border rect above it that touches no page.
    let items = vec![
        Item::canvas(page(0.0, 100.0)),
        Item::canvas(rect(0.0, 50.0, 200.0, 20.0)),
        Item::canvas(page(300.0, 100.0)),
        Item::canvas(rect(300.0, 50.0, 200.0, 20.0)),
    ];
    assert_eq!(
        batches(items),
        [
            (Prim::Page, vec![0]),
            (Prim::Shape, vec![1, 3]),
            (Prim::Page, vec![2])
        ]
    );
}

#[test]
fn pages_never_share_a_batch() {
    let items = vec![Item::canvas(page(0.0, 0.0)), Item::canvas(page(300.0, 0.0))];
    assert_eq!(
        batches(items),
        [(Prim::Page, vec![0]), (Prim::Page, vec![1])]
    );
}

#[test]
fn a_run_of_one_kind_stays_one_batch_even_when_it_overlaps_itself() {
    let items = vec![
        Item::canvas(rect(0.0, 0.0, 50.0, 50.0)),
        Item::canvas(rect(10.0, 10.0, 50.0, 50.0)),
        Item::canvas(rect(20.0, 20.0, 50.0, 50.0)),
    ];
    assert_eq!(batches(items), [(Prim::Shape, vec![0, 1, 2])]);
}

#[test]
fn an_item_joins_the_batch_it_overlaps_when_that_batch_is_its_own_kind() {
    let items = vec![
        Item::canvas(rect(0.0, 0.0, 50.0, 50.0)),
        Item::canvas(text(300.0, 300.0, 14.0)),
        Item::canvas(rect(10.0, 10.0, 50.0, 50.0)),
    ];
    assert_eq!(
        batches(items),
        [(Prim::Shape, vec![0, 2]), (CANVAS_TEXT, vec![1])]
    );
}

#[test]
fn canvas_and_screen_text_are_separate_batches() {
    let items = vec![
        Item::canvas(text(0.0, 0.0, 14.0)),
        Item::screen(text(300.0, 300.0, 14.0)),
    ];
    assert_eq!(
        batches(items),
        [(CANVAS_TEXT, vec![0]), (Prim::Text(Space::Screen), vec![1])]
    );
}

#[test]
fn shapes_under_different_clips_are_separate_batches_with_their_scissors() {
    let clip = Rect::new(0.0, 0.0, 40.0, 40.0);
    let items = vec![
        Item::canvas(rect(0.0, 0.0, 50.0, 50.0)).clipped(clip),
        Item::canvas(rect(300.0, 0.0, 50.0, 50.0)),
    ];
    let view = view(Vec2::ZERO, 1.0);
    let (placed, _) = placed(&view, items);
    let scissors: Vec<_> = batch(&placed, &view)
        .iter()
        .map(|batch| batch.scissor.map(|s| (s.width, s.height)))
        .collect();
    assert_eq!(scissors, [Some((40, 40)), None]);
}

#[test]
fn clipped_text_keeps_the_whole_target_as_its_scissor() {
    let clip = Rect::new(0.0, 0.0, 40.0, 40.0);
    let items = vec![
        Item::canvas(text(0.0, 0.0, 14.0)).clipped(clip),
        Item::canvas(text(300.0, 0.0, 14.0)),
    ];
    let view = view(Vec2::ZERO, 1.0);
    let (placed, _) = placed(&view, items);
    let batches = batch(&placed, &view);
    assert_eq!((batches.len(), batches[0].scissor), (1, None));
}
