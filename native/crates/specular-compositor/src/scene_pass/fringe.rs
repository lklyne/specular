//! Anti-aliased outlines for tessellated meshes. Pure.
//!
//! The scene pass draws one sample a pixel, so a mesh carries its own
//! anti-aliasing: every outline edge of a tessellated fill or stroke gets a
//! band one device pixel wide outside it, whose coverage falls from full on
//! the edge to none. The mesher tessellates each fill and stroke half a
//! device pixel inside its true outline ([`super::inset`]), so the band
//! straddles the true edge with half coverage on it, as multisampling gives,
//! and a shape keeps its weight. No vertex inside is moved, so no triangle
//! can turn over and cover the band twice. Only outline edges get a band:
//! an edge two triangles share has none, so nothing inside a path is drawn
//! twice, which a translucent or multiplied fill would show.
//!
//! The band is a device pixel wide at any zoom. Its outer vertices carry the
//! direction they move (`extrude`, in device pixels) apart from their
//! position, and the shader divides it by the scale factor, so a mesh held
//! at one zoom and drawn at another keeps its band.

use glam::Vec2;
use rustc_hash::FxHashMap;

use super::mesh::Mesh;
use crate::gpu_types::MeshVertex;

/// Vertices closer than this, in device pixels, are one vertex. lyon emits
/// the same point twice at some joins, and an edge between two copies would
/// read as outline.
const WELD_PX: f32 = 1.0 / 64.0;
/// A triangle lower than this, in logical pixels, has no area.
const FLAT: f32 = 1.0 / 256.0;
/// The longest a corner's miter may grow, in band widths. A sharper corner
/// gets a narrower band at its tip instead of a spike.
pub(crate) const MITER_LIMIT: f32 = 2.0;

/// Gives the triangles appended to `mesh` from `first_vertex` and
/// `first_index` a feathered outline. All of them must share one colour, as
/// one tessellation's do. `device_pixel` is a device pixel in the mesh's
/// units.
pub(crate) fn feather(mesh: &mut Mesh, first_vertex: usize, first_index: usize, device_pixel: f32) {
    weld(mesh, first_vertex, first_index, WELD_PX * device_pixel);
    prune(mesh, first_index);
    let edges = outline(mesh, first_index);
    let local = |at: u32| at as usize - first_vertex;
    let mut corners = vec![Corner::default(); mesh.vertices.len() - first_vertex];
    for edge in &edges {
        for end in [edge.from, edge.to] {
            corners[local(end)].add(edge.normal);
        }
    }
    // Where exactly two outline edges meet, they share an outer vertex
    // along the corner's miter. Any other corner (a point where two parts of
    // the outline touch) gives each edge an outer vertex of its own.
    let shared: Vec<Option<u32>> = (corners.iter().enumerate())
        .map(|(offset, corner)| {
            let miter = corner.miter()?;
            Some(outer(mesh, (first_vertex + offset) as u32, miter))
        })
        .collect();
    for edge in &edges {
        let mut out = |at: u32| shared[local(at)].unwrap_or_else(|| outer(mesh, at, edge.normal));
        let (from_out, to_out) = (out(edge.from), out(edge.to));
        mesh.indices
            .extend([edge.from, edge.to, to_out, edge.from, to_out, from_out]);
    }
}

/// Adds the band's outer vertex for the outline vertex `at`: moved a device
/// pixel along `out`, with no coverage.
fn outer(mesh: &mut Mesh, at: u32, out: Vec2) -> u32 {
    let vertex = MeshVertex {
        extrude: out.to_array(),
        coverage: 0.0,
        ..mesh.vertices[at as usize]
    };
    mesh.vertices.push(vertex);
    (mesh.vertices.len() - 1) as u32
}

/// Points every index at the first vertex in its cell of a grid
/// `tolerance` wide. lyon's copies of a point are the same point, so a grid
/// finds them with one lookup a vertex.
fn weld(mesh: &mut Mesh, first_vertex: usize, first_index: usize, tolerance: f32) {
    let mut grid: FxHashMap<(i32, i32), u32> = FxHashMap::default();
    grid.reserve(mesh.vertices.len() - first_vertex);
    let canonical: Vec<u32> = (mesh.vertices[first_vertex..].iter().enumerate())
        .map(|(offset, vertex)| {
            let [x, y] = vertex
                .position
                .map(|along| (along / tolerance).round() as i32);
            *grid.entry((x, y)).or_insert((first_vertex + offset) as u32)
        })
        .collect();
    for index in &mut mesh.indices[first_index..] {
        *index = canonical[*index as usize - first_vertex];
    }
}

/// Drops the triangles with no area. lyon emits them along straight runs of
/// an outline. They draw nothing, but counted they would hide the outline
/// edge of the real triangle beside them.
fn prune(mesh: &mut Mesh, first_index: usize) {
    let position = |at: u32| Vec2::from_array(mesh.vertices[at as usize].position);
    let kept: Vec<u32> = mesh.indices[first_index..]
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|t| {
            let (a, b, c) = (position(t[0]), position(t[1]), position(t[2]));
            let longest = (b - a).length().max((c - b).length()).max((a - c).length());
            // Twice the area over the longest side: the triangle's height.
            (b - a).perp_dot(c - a).abs() > FLAT * longest
        })
        .flatten()
        .copied()
        .collect();
    mesh.indices.truncate(first_index);
    mesh.indices.extend(kept);
}

/// An outline edge and the unit normal pointing away from its triangle.
struct Edge {
    from: u32,
    to: u32,
    normal: Vec2,
}

/// The edges only one triangle has, in triangle order.
fn outline(mesh: &Mesh, first_index: usize) -> Vec<Edge> {
    let position = |at: u32| Vec2::from_array(mesh.vertices[at as usize].position);
    let triangles = || {
        mesh.indices[first_index..]
            .as_chunks::<3>()
            .0
            .iter()
            .copied()
    };
    let key = |a: u32, b: u32| (a.min(b), a.max(b));
    let mut uses: FxHashMap<(u32, u32), u32> = FxHashMap::default();
    uses.reserve(mesh.indices.len() - first_index);
    for [a, b, c] in triangles() {
        for (p, q) in [(a, b), (b, c), (c, a)] {
            *uses.entry(key(p, q)).or_default() += 1;
        }
    }
    let mut edges = Vec::new();
    for [a, b, c] in triangles() {
        for (from, to, opposite) in [(a, b, c), (b, c, a), (c, a, b)] {
            if uses.get(&key(from, to)) != Some(&1) {
                continue;
            }
            let start = position(from);
            let normal = (position(to) - start).perp().normalize_or_zero();
            let inward = normal.dot(position(opposite) - start) > 0.0;
            edges.push(Edge {
                from,
                to,
                normal: if inward { -normal } else { normal },
            });
        }
    }
    edges
}

/// The outline edges meeting at one vertex.
#[derive(Default, Clone, Copy)]
struct Corner {
    normals: [Vec2; 2],
    count: u32,
}

impl Corner {
    fn add(&mut self, normal: Vec2) {
        if let Some(slot) = self.normals.get_mut(self.count as usize) {
            *slot = normal;
        }
        self.count += 1;
    }

    /// The outward direction that moves both edges by one unit; `None` unless
    /// exactly two edges meet here and they do not fold back on each other.
    fn miter(&self) -> Option<Vec2> {
        if self.count != 2 {
            return None;
        }
        let [first, second] = self.normals;
        let direction = (first + second).try_normalize()?;
        let cos = direction.dot(first).max(1.0 / MITER_LIMIT);
        Some(direction / cos)
    }
}
