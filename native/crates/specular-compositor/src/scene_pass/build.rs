//! Turns batches into what the pass draws: quad and shape instances, mesh
//! triangles, and the ordered list of draw calls. Pure; text batches only
//! get their slot here and are laid out by the text system.

use std::ops::Range;

use glam::Vec2;
use specular_core::{CssSize, PageId};
use specular_scene::OwnerId;
use specular_scene::{Blend, Draw, FramePin, ImageId, Item, PageDraw, Rect, Scene, Space};

use super::batch::Batch;
use super::mesh::{Mesh, Mesher};
use super::mesh_cache::MeshCache;
use super::place::{Placed, Prim, Scissor, ViewTransform};
use super::shapes::shape_instance;
use crate::draw_list::{DrawCounts, DrawItem, LayerKind, PageLayersInfo, popup_quad};
use crate::gpu_types::{QuadInstance, ShapeInstance};

/// The texture a quad samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuadTexture {
    Page(PageId, LayerKind),
    Image(ImageId),
}

/// One draw call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Op {
    /// Quad `instance` of the quad buffer.
    Quad { instance: u32, texture: QuadTexture },
    /// A range of the shape instance buffer.
    Shapes(Range<u32>),
    /// A range of the mesh index buffer, and how it meets the target.
    Mesh(Range<u32>, Blend),
    /// The glyphs of the `slot`th text batch of `space`.
    Text { slot: usize, space: Space },
}

/// A draw call and the scissor it runs under (`None` is the whole target).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawOp {
    pub(crate) scissor: Option<Scissor>,
    pub(crate) op: Op,
}

/// Where [`build`] writes. Every list is cleared first.
#[derive(Debug)]
pub(crate) struct Output<'a> {
    pub(crate) quads: &'a mut Vec<QuadInstance>,
    pub(crate) shapes: &'a mut Vec<ShapeInstance>,
    pub(crate) mesher: &'a mut Mesher,
    pub(crate) meshes: &'a mut MeshCache,
    pub(crate) mesh: &'a mut Mesh,
    /// The page layers drawn, for paint-to-submit latency.
    pub(crate) page_layers: &'a mut Vec<DrawItem>,
    pub(crate) draws: &'a mut Vec<DrawOp>,
}

impl Output<'_> {
    /// Empties every list and starts the mesh cache's frame.
    fn clear(&mut self) {
        self.quads.clear();
        self.shapes.clear();
        self.mesh.vertices.clear();
        self.mesh.indices.clear();
        self.page_layers.clear();
        self.draws.clear();
        self.meshes.begin_frame();
    }
}

/// Builds the draw calls for `batches`, in order.
///
/// `page` resolves a page entity, laid out at the CSS size given, to its
/// host's id and what the compositor holds to draw it from, `None` until it
/// has painted. `has_image` says whether an
/// image has been uploaded. A page or image with nothing to show is skipped.
pub(crate) fn build(
    scene: &Scene,
    placed: &[Placed],
    batches: &[Batch],
    view: &ViewTransform,
    page: impl Fn(&OwnerId, CssSize) -> Option<(PageId, PageLayersInfo)>,
    has_image: impl Fn(ImageId) -> bool,
    out: &mut Output<'_>,
) -> DrawCounts {
    out.clear();
    let mut counts = DrawCounts::default();
    // Text batches are numbered within their space.
    let (mut canvas_slots, mut screen_slots) = (0, 0);
    for batch in batches {
        let items = || {
            batch
                .members
                .iter()
                .map(|&at| &scene.items[placed[at].item])
        };
        let mut push = |op| {
            out.draws.push(DrawOp {
                scissor: batch.scissor,
                op,
            });
        };
        match batch.prim {
            Prim::Page => page_batch(items(), batch.scissor, view, &page, &mut counts, out),
            Prim::Image => image_batch(items(), batch.scissor, view, &has_image, out),
            Prim::Shape => {
                let start = out.shapes.len() as u32;
                out.shapes
                    .extend(items().filter_map(|item| shape_instance(item, view)));
                push(Op::Shapes(start..out.shapes.len() as u32));
            }
            Prim::Mesh(blend) => {
                let start = out.mesh.indices.len() as u32;
                for item in items() {
                    out.meshes.add(out.mesher, out.mesh, item, view);
                }
                let end = out.mesh.indices.len() as u32;
                if end > start {
                    push(Op::Mesh(start..end, blend));
                }
            }
            Prim::Text(space) => {
                let slots = match space {
                    Space::Canvas => &mut canvas_slots,
                    Space::Screen => &mut screen_slots,
                };
                push(Op::Text {
                    slot: *slots,
                    space,
                });
                *slots += 1;
            }
        }
    }
    counts
}

/// The quads of a batch of pages, each layer that has painted.
fn page_batch<'a>(
    items: impl Iterator<Item = &'a Item>,
    scissor: Option<Scissor>,
    view: &ViewTransform,
    page: &impl Fn(&OwnerId, CssSize) -> Option<(PageId, PageLayersInfo)>,
    counts: &mut DrawCounts,
    out: &mut Output<'_>,
) {
    for item in items {
        let Draw::Page(draw) = &item.draw else {
            continue;
        };
        let Some((id, info)) = page(&draw.page, draw.viewport) else {
            counts.pages_without_texture += 1;
            continue;
        };
        counts.cpu_textures += u32::from(info.view_is_cpu);
        note_drawn(id, draw, &info);
        let layers = page_quads(item, draw, &info, view);
        for (layer, quad) in layers {
            let Some(quad) = quad else {
                continue;
            };
            let instance = out.quads.len() as u32;
            out.quads.push(quad.with_opacity(item.opacity.min(1.0)));
            out.page_layers.push(DrawItem { page: id, layer });
            out.draws.push(DrawOp {
                scissor,
                op: Op::Quad {
                    instance,
                    texture: QuadTexture::Page(id, layer),
                },
            });
        }
    }
}

/// The quads of a batch of images, each one uploaded.
fn image_batch<'a>(
    items: impl Iterator<Item = &'a Item>,
    scissor: Option<Scissor>,
    view: &ViewTransform,
    has_image: &impl Fn(ImageId) -> bool,
    out: &mut Output<'_>,
) {
    for item in items {
        let Draw::Image(draw) = &item.draw else {
            continue;
        };
        if !has_image(draw.image) {
            continue;
        }
        let source = draw.source;
        let instance = out.quads.len() as u32;
        let quad = canvas_quad(item, draw.rect, draw.corner_radius, view)
            .with_uv_rect([source.x, source.y, source.width, source.height])
            .with_opacity(item.opacity.min(1.0));
        out.quads.push(if item.turn == 0.0 {
            quad
        } else {
            quad.turned(item.turn)
        });
        out.draws.push(DrawOp {
            scissor,
            op: Op::Quad {
                instance,
                texture: QuadTexture::Image(draw.image),
            },
        });
    }
}

/// Tells the resize ledger, when it is on, what a page is drawn from.
fn note_drawn(id: PageId, draw: &PageDraw, info: &PageLayersInfo) {
    if specular_core::ledger::enabled() {
        specular_core::ledger::note_page(specular_core::ledger::PageDrawn {
            page: id.0,
            wanted: draw.viewport,
            drawn: info.view_css,
            texels: info.view_size,
        });
    }
}

/// The quad for a page or image at `rect`. The quad shader works in canvas
/// space, so a screen-space rect is unprojected.
/// The quads of a page: its view, and its popup when one is showing.
fn page_quads(
    item: &Item,
    draw: &PageDraw,
    info: &PageLayersInfo,
    view: &ViewTransform,
) -> [(LayerKind, Option<QuadInstance>); 2] {
    let quad = canvas_quad(item, draw.rect, draw.corner_radius, view);
    let origin = Vec2::new(quad.rect[0], quad.rect[1]);
    let rect_size = Vec2::new(quad.rect[2], quad.rect[3]);
    if item.turn != 0.0 {
        // The frame keeps its own size about the rect's centre, whole, and
        // the rect turned is what cuts it.
        let frame = frame_size(rect_size, draw.viewport, info.view_css).unwrap_or(rect_size);
        let quad = QuadInstance::new(
            origin + (rect_size - frame) / 2.0,
            frame,
            quad.corner_radius,
        )
        .with_clip(rect_size / 2.0, item.turn);
        return [(info.view, Some(quad)), (LayerKind::Popup, None)];
    }
    let shown = unstretched(rect_size, draw.viewport, info.view_css, draw.pin);
    let popup = popup_quad(origin + shown.frame_origin, shown.frame, info);
    let quad = QuadInstance::new(origin + shown.offset, shown.size, quad.corner_radius)
        .with_uv_rect(shown.uv);
    [(info.view, Some(quad)), (LayerKind::Popup, popup)]
}

/// The part of a page's frame a rect shows, in the rect's units from its
/// top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Shown {
    /// Where the part shown starts.
    offset: Vec2,
    /// How big it is.
    size: Vec2,
    /// Which part of the frame it is, `[x, y, width, height]` in fractions.
    uv: [f32; 4],
    /// Where the whole frame's corner sits, and how big the frame is.
    frame_origin: Vec2,
    frame: Vec2,
}

/// What a rect `size` across shows of a page's frame, when the rect is laid
/// out at `wanted` CSS pixels and the frame was painted at `painted`. A
/// frame of another size keeps its own pixel size, as a browser's does
/// while its window is dragged: the part past the rect is cut off, and the
/// part of the rect past the frame is left bare. It keeps to the corner
/// `pin` names, the one a resize holds still, so it does not slide with the
/// edge being dragged. A size nobody stated fills the rect.
fn unstretched(size: Vec2, wanted: CssSize, painted: CssSize, pin: FramePin) -> Shown {
    let Some(frame) = frame_size(size, wanted, painted) else {
        return Shown {
            offset: Vec2::ZERO,
            size,
            uv: [0.0, 0.0, 1.0, 1.0],
            frame_origin: Vec2::ZERO,
            frame: size,
        };
    };
    let shown = frame.min(size);
    let toward = Vec2::new(
        f32::from(u8::from(pin.right)),
        f32::from(u8::from(pin.bottom)),
    );
    // Against the right or bottom edge, the frame's far side meets it.
    let frame_origin = (size - frame) * toward;
    let offset = (size - shown) * toward;
    let start = (offset - frame_origin) / frame;
    let part = shown / frame;
    Shown {
        offset,
        size: shown,
        uv: [start.x, start.y, part.x, part.y],
        frame_origin,
        frame,
    }
}

/// The size of a page's frame painted at `painted` CSS pixels, in the units
/// of a rect `size` across that is laid out at `wanted`. `None` when the
/// frame is the rect's own size, or a size was not stated.
fn frame_size(size: Vec2, wanted: CssSize, painted: CssSize) -> Option<Vec2> {
    let css = |size: CssSize| Vec2::new(size.width as f32, size.height as f32);
    let (wanted, painted) = (css(wanted), css(painted));
    let stated = wanted.min_element() > 0.0 && painted.min_element() > 0.0;
    (stated && wanted != painted).then(|| size * painted / wanted)
}

fn canvas_quad(item: &Item, rect: Rect, corner_radius: f32, view: &ViewTransform) -> QuadInstance {
    let origin = Vec2::new(rect.x, rect.y);
    let size = Vec2::new(rect.width, rect.height);
    let radius = corner_radius.max(0.0);
    match item.space {
        Space::Canvas => QuadInstance::new(origin, size, radius),
        Space::Screen => {
            let zoom = view.camera.zoom;
            QuadInstance::new(
                view.camera.screen_to_world(origin),
                size / zoom,
                radius / zoom,
            )
        }
    }
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
