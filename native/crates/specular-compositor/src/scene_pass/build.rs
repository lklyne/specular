//! Turns batches into what the pass draws: quad and shape instances, mesh
//! triangles, and the ordered list of draw calls. Pure; text batches only
//! get their slot here and are laid out by the text system.

use std::ops::Range;

use glam::Vec2;
use specular_core::PageId;
use specular_doc::EntityId;
use specular_scene::{Blend, Draw, ImageId, Item, Rect, Scene, Space};

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
/// `page` resolves a page entity to its host's id and what the compositor
/// holds for it, `None` until it has painted. `has_image` says whether an
/// image has been uploaded. A page or image with nothing to show is skipped.
pub(crate) fn build(
    scene: &Scene,
    placed: &[Placed],
    batches: &[Batch],
    view: &ViewTransform,
    page: impl Fn(&EntityId) -> Option<(PageId, PageLayersInfo)>,
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
            Prim::Page => {
                for item in items() {
                    let Draw::Page(draw) = &item.draw else {
                        continue;
                    };
                    let Some((id, info)) = page(&draw.page) else {
                        counts.pages_without_texture += 1;
                        continue;
                    };
                    counts.cpu_textures += u32::from(info.view_is_cpu);
                    let quad = canvas_quad(item, draw.rect, draw.corner_radius, view);
                    let popup = popup_quad(
                        Vec2::new(quad.rect[0], quad.rect[1]),
                        Vec2::new(quad.rect[2], quad.rect[3]),
                        &info,
                    );
                    let layers = [(LayerKind::View, Some(quad)), (LayerKind::Popup, popup)];
                    for (layer, quad) in layers {
                        let Some(quad) = quad else {
                            continue;
                        };
                        let instance = out.quads.len() as u32;
                        out.quads.push(quad.with_opacity(item.opacity.min(1.0)));
                        out.page_layers.push(DrawItem { page: id, layer });
                        push(Op::Quad {
                            instance,
                            texture: QuadTexture::Page(id, layer),
                        });
                    }
                }
            }
            Prim::Image => {
                for item in items() {
                    let Draw::Image(draw) = &item.draw else {
                        continue;
                    };
                    if !has_image(draw.image) {
                        continue;
                    }
                    let source = draw.source;
                    let instance = out.quads.len() as u32;
                    out.quads.push(
                        canvas_quad(item, draw.rect, draw.corner_radius, view)
                            .with_uv_rect([source.x, source.y, source.width, source.height])
                            .with_opacity(item.opacity.min(1.0)),
                    );
                    push(Op::Quad {
                        instance,
                        texture: QuadTexture::Image(draw.image),
                    });
                }
            }
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

/// The quad for a page or image at `rect`. The quad shader works in canvas
/// space, so a screen-space rect is unprojected.
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
