//! Tessellated meshes kept between frames.
//!
//! Tessellating a stroke costs far more than drawing it, and a stroke on the
//! canvas does not change when the camera moves: a pan slides it and a zoom
//! scales it. So each canvas polygon and path is tessellated once, with no
//! pan and at a zoom rounded up to a power of two, and every frame copies the
//! triangles into the frame's buffer under the camera. Tessellating at the
//! rounded zoom makes the mesh a little finer than the frame needs, never
//! coarser, and a zoom gesture tessellates again only when it crosses a
//! power of two.
//!
//! The key is a hash of what the triangles depend on: the geometry, the
//! colours, the opacity, the scale factor and the tessellation zoom. The
//! scene carries no identity for an item, and needs none here.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use glam::Vec2;
use rustc_hash::FxHasher;
use specular_core::Camera;
use specular_scene::{Draw, Item, PathCommand, PathStroke, Point, Space};

use super::mesh::{Mesh, Mesher};
use super::place::ViewTransform;
use crate::gpu_types::MeshVertex;

/// Frames a mesh outlives the last frame that drew it.
const KEEP_FRAMES: u64 = 240;

/// What the cache did in one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeshCacheCounts {
    /// Items drawn from a mesh already held.
    pub hits: u32,
    /// Items tessellated this frame and kept.
    pub misses: u32,
    /// Items tessellated this frame and not kept: screen-space ones, whose
    /// geometry is the camera's.
    pub uncached: u32,
}

#[derive(Debug)]
struct Entry {
    /// Positions are canvas points times `zoom`, with no pan.
    mesh: Mesh,
    /// The zoom it was tessellated at.
    zoom: f32,
    last_used: u64,
}

/// The meshes of the canvas items drawn lately.
#[derive(Debug, Default)]
pub(crate) struct MeshCache {
    entries: HashMap<u64, Entry>,
    frame: u64,
    counts: MeshCacheCounts,
}

impl MeshCache {
    /// Starts a frame: drops what has not been drawn for a while.
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
        self.counts = MeshCacheCounts::default();
        let frame = self.frame;
        self.entries
            .retain(|_, entry| frame - entry.last_used <= KEEP_FRAMES);
    }

    /// What this frame hit and missed so far.
    pub(crate) fn counts(&self) -> MeshCacheCounts {
        self.counts
    }

    /// Appends the triangles of a polygon or path item to `mesh`, in logical
    /// screen pixels, as [`Mesher::add`] does, tessellating only if no held
    /// mesh fits.
    pub(crate) fn add(
        &mut self,
        mesher: &mut Mesher,
        mesh: &mut Mesh,
        item: &Item,
        view: &ViewTransform,
    ) {
        if item.space == Space::Screen {
            self.counts.uncached += 1;
            mesher.add(mesh, item, view);
            return;
        }
        let zoom = tessellation_zoom(item, view);
        let Some(key) = key(item, zoom, view.scale_factor) else {
            return;
        };
        let frame = self.frame;
        let entry = match self.entries.entry(key) {
            std::collections::hash_map::Entry::Occupied(held) => {
                self.counts.hits += 1;
                held.into_mut()
            }
            std::collections::hash_map::Entry::Vacant(empty) => {
                self.counts.misses += 1;
                // Built by hand: the tessellation zoom can round up past
                // the largest zoom a camera may have, and `Camera::new`
                // would clamp it.
                let unpanned = ViewTransform {
                    camera: Camera {
                        pan: Vec2::ZERO,
                        zoom,
                    },
                    ..*view
                };
                let mut tessellated = Mesh::new();
                mesher.add(&mut tessellated, item, &unpanned);
                empty.insert(Entry {
                    mesh: tessellated,
                    zoom,
                    last_used: frame,
                })
            }
        };
        entry.last_used = frame;
        let scale = view.camera.zoom / entry.zoom;
        let pan = view.camera.pan;
        let base = mesh.vertices.len() as u32;
        mesh.vertices
            .extend(entry.mesh.vertices.iter().map(|vertex| MeshVertex {
                position: [
                    vertex.position[0] * scale + pan.x,
                    vertex.position[1] * scale + pan.y,
                ],
                color: vertex.color,
            }));
        mesh.indices
            .extend(entry.mesh.indices.iter().map(|index| index + base));
    }
}

/// The zoom to tessellate `item` at for this frame: the camera's zoom
/// rounded up to a power of two. A stroke thinner than a device pixel is
/// drawn at exactly one pixel and faded, which is not a scaling of anything,
/// so it is tessellated at the camera's own zoom.
fn tessellation_zoom(item: &Item, view: &ViewTransform) -> f32 {
    let zoom = view.camera.zoom;
    let stroke = match &item.draw {
        Draw::Polygon(polygon) => polygon.stroke,
        Draw::Path(path) => path.stroke,
        Draw::Page(_)
        | Draw::Shadow(_)
        | Draw::Rect(_)
        | Draw::Ellipse(_)
        | Draw::Text(_)
        | Draw::Column(_)
        | Draw::Image(_) => None,
    };
    let device_pixel = 1.0 / view.scale_factor.max(1.0);
    let hairline = stroke.is_some_and(|stroke| stroke.width * zoom < device_pixel);
    if hairline || !(zoom.is_finite() && zoom > 0.0) {
        return zoom;
    }
    2.0_f32.powf(zoom.log2().ceil())
}

/// A hash of everything the triangles of `item` depend on; `None` for a
/// draw that has no mesh.
fn key(item: &Item, zoom: f32, scale_factor: f32) -> Option<u64> {
    let mut hasher = FxHasher::default();
    zoom.to_bits().hash(&mut hasher);
    scale_factor.to_bits().hash(&mut hasher);
    item.opacity.to_bits().hash(&mut hasher);
    let (fill, stroke) = match &item.draw {
        Draw::Polygon(polygon) => {
            0_u8.hash(&mut hasher);
            polygon.points.len().hash(&mut hasher);
            for &at in &polygon.points {
                hash_point(at, &mut hasher);
            }
            (polygon.fill, polygon.stroke)
        }
        Draw::Path(path) => {
            1_u8.hash(&mut hasher);
            path.commands.len().hash(&mut hasher);
            for command in &path.commands {
                hash_command(command, &mut hasher);
            }
            (path.fill, path.stroke)
        }
        Draw::Page(_)
        | Draw::Shadow(_)
        | Draw::Rect(_)
        | Draw::Ellipse(_)
        | Draw::Text(_)
        | Draw::Column(_)
        | Draw::Image(_) => return None,
    };
    fill.hash(&mut hasher);
    hash_stroke(stroke.as_ref(), &mut hasher);
    Some(hasher.finish())
}

fn hash_point(at: Point, hasher: &mut FxHasher) {
    (u64::from(at.x.to_bits()) << 32 | u64::from(at.y.to_bits())).hash(hasher);
}

fn hash_command(command: &PathCommand, hasher: &mut FxHasher) {
    match *command {
        PathCommand::MoveTo(to) => {
            0_u8.hash(hasher);
            hash_point(to, hasher);
        }
        PathCommand::LineTo(to) => {
            1_u8.hash(hasher);
            hash_point(to, hasher);
        }
        PathCommand::QuadTo { control, to } => {
            2_u8.hash(hasher);
            hash_point(control, hasher);
            hash_point(to, hasher);
        }
        PathCommand::CubicTo {
            control1,
            control2,
            to,
        } => {
            3_u8.hash(hasher);
            hash_point(control1, hasher);
            hash_point(control2, hasher);
            hash_point(to, hasher);
        }
        PathCommand::Close => 4_u8.hash(hasher),
    }
}

fn hash_stroke(stroke: Option<&PathStroke>, hasher: &mut FxHasher) {
    let Some(stroke) = stroke else {
        0_u8.hash(hasher);
        return;
    };
    1_u8.hash(hasher);
    stroke.color.hash(hasher);
    stroke.width.to_bits().hash(hasher);
    stroke.cap.hash(hasher);
    stroke.join.hash(hasher);
    stroke
        .dash
        .map(|dash| (dash.on.to_bits(), dash.off.to_bits()))
        .hash(hasher);
}

#[cfg(test)]
#[path = "mesh_cache_tests.rs"]
mod tests;
