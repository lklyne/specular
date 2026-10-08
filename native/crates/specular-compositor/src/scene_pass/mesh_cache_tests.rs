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

fn direct(items: &[Item], pan: Vec2, zoom: f32) -> Mesh {
    let mut mesh = Mesh::new();
    let mut mesher = Mesher::default();
    for item in items {
        mesher.add(&mut mesh, item, &view(pan, zoom));
    }
    mesh
}

fn close(a: &Mesh, b: &Mesh) -> bool {
    a.indices == b.indices
        && a.vertices.len() == b.vertices.len()
        && a.vertices.iter().zip(&b.vertices).all(|(a, b)| {
            a.color.map(f32::to_bits) == b.color.map(f32::to_bits)
                && (a.position[0] - b.position[0]).abs() < 1e-3
                && (a.position[1] - b.position[1]).abs() < 1e-3
        })
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

#[test]
fn a_pan_reuses_the_mesh_and_lands_where_tessellating_afresh_would() {
    let mut cache = MeshCache::default();
    let items = [stroke(4.0), triangle()];
    frame(&mut cache, &items, Vec2::ZERO, 1.0);
    let pan = Vec2::new(-37.5, 220.0);
    let (panned, did) = frame(&mut cache, &items, pan, 1.0);
    assert_eq!(did, counts(2, 0));
    assert!(close(&panned, &direct(&items, pan, 1.0)));
}

#[test]
fn a_zoom_reuses_the_mesh_until_it_passes_a_power_of_two() {
    let mut cache = MeshCache::default();
    let items = [stroke(4.0)];
    // Tessellated at zoom 1, the power of two at or above 0.6.
    assert_eq!(frame(&mut cache, &items, Vec2::ZERO, 0.6).1, counts(0, 1));
    for zoom in [0.7, 0.85, 1.0, 0.51] {
        let (mesh, did) = frame(&mut cache, &items, Vec2::new(5.0, 9.0), zoom);
        assert_eq!(did, counts(1, 0), "zoom {zoom}");
        // The same outline, scaled: each vertex within the flattening
        // tolerance of where a fresh tessellation puts the stroke's edge.
        let fresh = direct(&items, Vec2::new(5.0, 9.0), zoom);
        let extent = |mesh: &Mesh| {
            let at = mesh.vertices.iter().map(|v| Vec2::from(v.position));
            (
                at.clone().fold(Vec2::MAX, Vec2::min),
                at.fold(Vec2::MIN, Vec2::max),
            )
        };
        let (held, new) = (extent(&mesh), extent(&fresh));
        assert!(
            held.0.distance(new.0) < 0.3 && held.1.distance(new.1) < 0.3,
            "zoom {zoom}: {held:?} against {new:?}"
        );
    }
    assert_eq!(frame(&mut cache, &items, Vec2::ZERO, 1.2).1, counts(0, 1));
    assert_eq!(frame(&mut cache, &items, Vec2::ZERO, 0.4).1, counts(0, 1));
}

#[test]
fn a_changed_point_colour_or_opacity_is_tessellated_again() {
    let mut cache = MeshCache::default();
    frame(&mut cache, &[stroke(4.0)], Vec2::ZERO, 1.0);
    let wider = stroke(5.0);
    let faded = Item {
        opacity: 0.5,
        ..stroke(4.0)
    };
    let mut moved = stroke(4.0);
    if let Draw::Path(path) = &mut moved.draw {
        path.commands[1] = PathCommand::LineTo(Point::new(61.0, 10.0));
    }
    for changed in [wider, faded, moved] {
        let (_, did) = frame(&mut cache, std::slice::from_ref(&changed), Vec2::ZERO, 1.0);
        assert_eq!(did, counts(0, 1));
    }
    // The first is still held.
    assert_eq!(
        frame(&mut cache, &[stroke(4.0)], Vec2::ZERO, 1.0).1,
        counts(1, 0)
    );
}

#[test]
fn a_hairline_is_tessellated_for_each_zoom_and_once_for_a_pan() {
    let mut cache = MeshCache::default();
    // One canvas unit wide: under a pixel at any zoom below 1.
    let items = [stroke(1.0)];
    assert_eq!(frame(&mut cache, &items, Vec2::ZERO, 0.30).1, counts(0, 1));
    assert_eq!(frame(&mut cache, &items, Vec2::ZERO, 0.31).1, counts(0, 1));
    let pan = Vec2::new(12.0, -4.0);
    let (panned, did) = frame(&mut cache, &items, pan, 0.31);
    assert_eq!(did, counts(1, 0));
    assert!(close(&panned, &direct(&items, pan, 0.31)));
}

#[test]
fn a_screen_item_is_tessellated_every_frame_and_never_kept() {
    let mut cache = MeshCache::default();
    let item = Item::screen(PathDraw::polyline(
        [Point::new(0.0, 0.0), Point::new(40.0, 40.0)],
        PathStroke::new(RED, 2.0),
    ));
    for _ in 0..2 {
        let (mesh, did) = frame(&mut cache, std::slice::from_ref(&item), Vec2::ZERO, 1.0);
        assert_eq!((did.hits, did.misses, did.uncached), (0, 0, 1));
        assert!(mesh.indices.len() >= 6);
    }
}

#[test]
fn a_mesh_not_drawn_for_a_while_is_dropped() {
    let mut cache = MeshCache::default();
    frame(&mut cache, &[stroke(4.0)], Vec2::ZERO, 1.0);
    for _ in 0..=KEEP_FRAMES {
        frame(&mut cache, &[], Vec2::ZERO, 1.0);
    }
    assert_eq!(
        frame(&mut cache, &[stroke(4.0)], Vec2::ZERO, 1.0).1,
        counts(0, 1)
    );
}
