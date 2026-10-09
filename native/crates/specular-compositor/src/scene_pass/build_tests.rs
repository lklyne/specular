use specular_core::{CssSize, PixelRect, PixelSize};
use specular_scene::{ImageDraw, PageDraw};

use super::super::batch::batch;
use super::super::place::tests::{placed, rect, text, view};
use super::*;

const PAGE: PageId = PageId(7);

struct Built {
    quads: Vec<QuadInstance>,
    shapes: Vec<ShapeInstance>,
    page_layers: Vec<DrawItem>,
    ops: Vec<Op>,
    counts: DrawCounts,
}

fn built(zoom: f32, items: Vec<Item>, info: Option<PageLayersInfo>) -> Built {
    let view = view(Vec2::ZERO, zoom);
    let scene = Scene::from_iter(items.clone());
    let (placed, _) = placed(&view, items);
    let batches = batch(&placed, &view);
    let (mut quads, mut shapes, mut page_layers, mut draws) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let counts = build(
        &scene,
        &placed,
        &batches,
        &view,
        |_| info.map(|info| (PAGE, info)),
        |image| image == ImageId(1),
        &mut Output {
            quads: &mut quads,
            shapes: &mut shapes,
            mesher: &mut Mesher::default(),
            meshes: &mut MeshCache::default(),
            mesh: &mut Mesh::new(),
            page_layers: &mut page_layers,
            draws: &mut draws,
        },
    );
    Built {
        quads,
        shapes,
        page_layers,
        ops: draws.into_iter().map(|draw| draw.op).collect(),
        counts,
    }
}

fn page() -> PageDraw {
    PageDraw {
        page: OwnerId::new("page"),
        rect: Rect::new(10.0, 0.0, 200.0, 100.0),
        viewport: CssSize::new(200, 100),
        corner_radius: 8.0,
    }
}

fn painted(popup: Option<PixelRect>) -> PageLayersInfo {
    PageLayersInfo {
        view_size: PixelSize::new(400, 200),
        view_css: CssSize::new(200, 100),
        view_is_cpu: true,
        popup,
    }
}

#[test]
fn a_painted_page_is_one_quad_sampling_its_view() {
    let built = built(1.0, vec![Item::canvas(page())], Some(painted(None)));
    let texture = QuadTexture::Page(PAGE, LayerKind::View);
    assert_eq!(
        (built.ops, built.quads[0].rect, built.counts.cpu_textures),
        (
            vec![Op::Quad {
                instance: 0,
                texture
            }],
            [10.0, 0.0, 200.0, 100.0],
            1
        )
    );
}

#[test]
fn a_popup_is_a_second_quad_over_its_page() {
    // The view texture is 2x the canvas rect, so popup texels halve.
    let info = painted(Some(PixelRect::new(40, 20, 100, 60)));
    let built = built(1.0, vec![Item::canvas(page())], Some(info));
    let layers: Vec<_> = built.page_layers.iter().map(|item| item.layer).collect();
    assert_eq!(
        (layers, built.quads[1].rect),
        (
            vec![LayerKind::View, LayerKind::Popup],
            [30.0, 10.0, 50.0, 30.0]
        )
    );
}

#[test]
fn draws_follow_batch_order_and_text_batches_are_numbered() {
    let items = vec![
        Item::canvas(rect(50.0, 50.0, 20.0, 20.0)),
        Item::canvas(page()),
        Item::canvas(rect(60.0, 60.0, 20.0, 20.0)),
        Item::canvas(text(60.0, 60.0, 14.0)),
        Item::canvas(rect(60.0, 60.0, 20.0, 20.0)),
        Item::canvas(text(60.0, 60.0, 14.0)),
    ];
    let built = built(1.0, items, Some(painted(None)));
    let texture = QuadTexture::Page(PAGE, LayerKind::View);
    assert_eq!(
        built.ops,
        [
            Op::Shapes(0..1),
            Op::Quad {
                instance: 0,
                texture
            },
            Op::Shapes(1..2),
            Op::Text {
                slot: 0,
                space: Space::Canvas
            },
            Op::Shapes(2..3),
            Op::Text {
                slot: 1,
                space: Space::Canvas
            }
        ]
    );
    assert_eq!(built.shapes.len(), 3);
}

#[test]
fn a_screen_space_image_is_unprojected_into_the_quad_shaders_space() {
    let image = ImageDraw {
        source: Rect::new(0.25, 0.0, 0.5, 1.0),
        corner_radius: 4.0,
        ..ImageDraw::new(ImageId(1), Rect::new(100.0, 40.0, 60.0, 30.0))
    };
    let built = built(2.0, vec![Item::screen(image).with_opacity(0.5)], None);
    let quad = built.quads[0];
    assert_eq!(
        (quad.rect, quad.uv_rect, quad.corner_radius, quad.opacity),
        ([50.0, 20.0, 30.0, 15.0], [0.25, 0.0, 0.5, 1.0], 2.0, 0.5)
    );
}

#[test]
fn a_frame_of_another_size_keeps_its_pixel_size_from_the_corner() {
    // The rect is laid out at 200 by 100 CSS pixels and 200 by 100 across.
    let rows = [
        // Painted narrower and taller: bare on the right, cut at the bottom.
        (
            CssSize::new(100, 200),
            [10.0, 0.0, 100.0, 100.0],
            [0.0, 0.0, 1.0, 0.5],
        ),
        // Painted larger both ways: the top-left of the frame fills the rect.
        (
            CssSize::new(400, 400),
            [10.0, 0.0, 200.0, 100.0],
            [0.0, 0.0, 0.5, 0.25],
        ),
        // Painted at the size asked for: all of it, over all of the rect.
        (
            CssSize::new(200, 100),
            [10.0, 0.0, 200.0, 100.0],
            [0.0, 0.0, 1.0, 1.0],
        ),
    ];
    for (view_css, rect, uv_rect) in rows {
        let info = PageLayersInfo {
            view_css,
            ..painted(None)
        };
        let built = built(1.0, vec![Item::canvas(page())], Some(info));
        assert_eq!(
            (built.quads[0].rect, built.quads[0].uv_rect),
            (rect, uv_rect),
            "{view_css:?}"
        );
    }
}
