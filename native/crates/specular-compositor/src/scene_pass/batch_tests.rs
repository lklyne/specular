use glam::Vec2;
use specular_scene::OwnerId;
use specular_scene::{Item, PageDraw, Scene, Size, Space, TextRun};

use super::super::place::place;
use super::super::place::tests::{placed, rect, text, view};
use super::*;

fn page(x: f32, y: f32) -> PageDraw {
    PageDraw {
        page: OwnerId::new("page"),
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

/// The batching rule written the plain way: an item joins the last batch
/// if that is its kind, and otherwise the earliest batch of its kind at or
/// after the last batch any of whose members it overlaps. What [`Batcher`] must agree with on every input.
fn plain(placed: &[Placed], view: &ViewTransform) -> Vec<(Prim, Vec<usize>)> {
    let mut batches: Vec<(Kind, Vec<usize>)> = Vec::new();
    for (index, item) in placed.iter().enumerate() {
        let scissor = match (item.prim, item.clip) {
            (Prim::Text(_), _) | (_, None) => None,
            (_, Some(clip)) => match view.scissor(clip) {
                Some(scissor) => Some(scissor),
                None => continue,
            },
        };
        let kind = (item.prim, scissor);
        // An item of the last batch's kind joins it without looking.
        let joins_last =
            (batches.last()).is_some_and(|(last, _)| shares(item.prim) && *last == kind);
        let target = if joins_last {
            Some(batches.len() - 1)
        } else {
            let floor = (batches.iter())
                .rposition(|(_, members)| {
                    (members.iter()).any(|&member| placed[member].bounds.intersects(item.bounds))
                })
                .unwrap_or(0);
            (floor..batches.len()).find(|&at| shares(item.prim) && batches[at].0 == kind)
        };
        match target {
            Some(at) => batches[at].1.push(index),
            None => batches.push((kind, vec![index])),
        }
    }
    (batches.into_iter())
        .map(|((prim, _), members)| (prim, members))
        .collect()
}

/// A scatter of notes, pages, clipped shapes and screen text that overlap
/// here and there, from a fixed seed.
fn scatter(count: usize, seed: u64) -> Vec<Item> {
    let mut state = seed;
    let mut next = |below: f32| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) as f32 / (1_u64 << 31) as f32) * below
    };
    (0..count)
        .map(|index| {
            let (x, y) = (next(760.0), next(560.0));
            match index % 7 {
                0 => Item::canvas(page(x, y)),
                1 | 2 => Item::canvas(text(x, y, 14.0)),
                3 => Item::screen(text(x, y, 12.0)),
                4 => Item::canvas(rect(x, y, 30.0, 30.0)).clipped(Rect::new(x, y, 20.0, 20.0)),
                _ => Item::canvas(rect(x, y, 10.0 + next(60.0), 10.0 + next(60.0))),
            }
        })
        .collect()
}

#[test]
fn the_grid_batches_exactly_as_the_plain_rule_does() {
    let view = view(Vec2::ZERO, 1.0);
    for (count, seed) in [(40, 1), (200, 2), (200, 3), (900, 4)] {
        let (placed, _) = placed(&view, scatter(count, seed));
        let grid: Vec<(Prim, Vec<usize>)> = batch(&placed, &view)
            .into_iter()
            .map(|batch| (batch.prim, batch.members))
            .collect();
        assert_eq!(grid, plain(&placed, &view), "{count} items, seed {seed}");
    }
}

#[test]
fn an_item_is_tested_against_its_neighbours_not_against_every_item_before_it() {
    // 40 by 30 notes that do not touch: 2,400 items, alternating a rect
    // and its text, which is the order that made the plain rule quadratic.
    let view = view(Vec2::ZERO, 1.0);
    let notes: Vec<Item> = (0..1_200)
        .flat_map(|index| {
            let (column, row) = ((index % 40) as f32, (index / 40) as f32);
            [
                Item::canvas(rect(column * 20.0, row * 20.0, 16.0, 16.0)),
                Item::canvas(TextRun {
                    wrap_width: Some(8.0),
                    ..text(column * 20.0 + 2.0, row * 20.0 + 2.0, 4.0)
                }),
            ]
        })
        .collect();
    let scene = Scene { items: notes };
    let mut placed = Vec::new();
    place(&scene, &view, |_| Size::new(8.0, 8.0), &mut placed);
    assert_eq!(placed.len(), 2_400);
    let mut batcher = Batcher::default();
    let batches = batcher.batch(&placed, &view);
    assert_eq!(batches.len(), 2);
    // The plain rule compares each item with every one before it: about
    // 2.9 million pairs. A cell holds a handful.
    let tests = batcher.overlap_tests();
    assert!(tests < 2_400 * 16, "{tests} overlap tests");
}
