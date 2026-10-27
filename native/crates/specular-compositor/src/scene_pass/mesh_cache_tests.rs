use specular_scene::{Color, PathDraw, PolygonDraw};

use super::super::place::tests::view;
use super::*;

const RED: Color = Color::rgb(255, 0, 0);

fn stroke(width: f32) -> Item {
    Item::canvas(PathDraw::polyline(
        [
            Point::new(0.0, 50.0),
            Point::new(60.0, 10.0),
            Point::new(100.0, 50.0),
        ],
        PathStroke::new(RED, width),
    ))
}

fn triangle() -> Item {
    Item::canvas(PolygonDraw {
        points: vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(0.0, 100.0),
        ],
        fill: Some(RED),
        stroke: None,
    })
}

/// Draws `items` as one frame and returns the mesh and what the cache did.
fn frame(cache: &mut MeshCache, items: &[Item], pan: Vec2, zoom: f32) -> (Mesh, MeshCacheCounts) {
    let mut mesh = Mesh::new();
    let mut mesher = Mesher::default();
    cache.begin_frame();
    for item in items {
        cache.add(&mut mesher, &mut mesh, item, &view(pan, zoom));
    }
    (mesh, cache.counts())
}

fn counts(hits: u32, misses: u32) -> MeshCacheCounts {
    MeshCacheCounts {
        hits,
        misses,
        uncached: 0,
    }
}

#[test]
fn a_second_frame_of_the_same_items_tessellates_nothing() {
    let mut cache = MeshCache::default();
    let items = [stroke(4.0), triangle()];
    let (first, did) = frame(&mut cache, &items, Vec2::ZERO, 1.0);
    assert_eq!(did, counts(0, 2));
    let (second, did) = frame(&mut cache, &items, Vec2::ZERO, 1.0);
    assert_eq!(did, counts(2, 0));
    assert_eq!(first.vertices, second.vertices);
    assert_eq!(first.indices, second.indices);
}
