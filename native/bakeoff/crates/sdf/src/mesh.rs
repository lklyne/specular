//! lyon turns strokes and arrows into one triangle mesh in canvas units.

use bakeoff_scene::{
    Camera, World,
    world::{self, Arrow, Stroke},
};
use bytemuck::{Pod, Zeroable};
use lyon::{
    math::point,
    path::Path,
    tessellation::{
        BuffersBuilder, LineCap, LineJoin, StrokeOptions, StrokeTessellator, StrokeVertex,
        VertexBuffers,
    },
};

/// Flattening error, in screen pixels.
const TOLERANCE_PX: f32 = 0.2;
/// Strokes thinner than this on screen break up under MSAA, so they are held here.
const MIN_WIDTH_PX: f32 = 1.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 2],
    color: [u8; 4],
}

pub type Mesh = VertexBuffers<Vertex, u32>;

pub struct Mesher {
    tessellator: StrokeTessellator,
    stroke_paths: Vec<Path>,
    arrow_paths: Vec<Path>,
}

impl Mesher {
    pub fn new(world: &World) -> Self {
        Self {
            tessellator: StrokeTessellator::new(),
            stroke_paths: world.strokes.iter().map(stroke_path).collect(),
            arrow_paths: world.arrows.iter().map(arrow_path).collect(),
        }
    }

    /// Tessellates for `zoom`: flattening and the minimum width both depend on
    /// it, so a mesh is only reusable while the zoom holds. `cull` limits the
    /// work to what that camera sees.
    pub fn tessellate(&mut self, mesh: &mut Mesh, world: &World, zoom: f32, cull: Option<Camera>) {
        mesh.vertices.clear();
        mesh.indices.clear();
        let sees = |points: &[[f32; 2]], pad| {
            let (origin, size) = world::bounds(points, pad);
            cull.is_none_or(|camera| camera.sees(origin, size))
        };
        let options = |width: f32| {
            StrokeOptions::tolerance(TOLERANCE_PX / zoom)
                .with_line_width(width.max(MIN_WIDTH_PX / zoom))
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round)
        };

        for (stroke, path) in world.strokes.iter().zip(&self.stroke_paths) {
            if sees(&stroke.points, stroke.width) {
                stroke_into(
                    &mut self.tessellator,
                    mesh,
                    path,
                    &options(stroke.width),
                    stroke.color,
                );
            }
        }
        for (arrow, path) in world.arrows.iter().zip(&self.arrow_paths) {
            if sees(&arrow.p, arrow.width * 5.0) {
                stroke_into(
                    &mut self.tessellator,
                    mesh,
                    path,
                    &options(arrow.width),
                    arrow.color,
                );
                let first = mesh.vertices.len() as u32;
                mesh.vertices
                    .extend(arrow.head().map(|at| vertex(at, arrow.color)));
                mesh.indices.extend([first, first + 1, first + 2]);
            }
        }
    }
}

fn stroke_into(
    tessellator: &mut StrokeTessellator,
    mesh: &mut Mesh,
    path: &Path,
    options: &StrokeOptions,
    color: [u8; 3],
) {
    // The only failure is a mesh past u32 indices, which this scene is nowhere near.
    let _ = tessellator.tessellate_path(
        path,
        options,
        &mut BuffersBuilder::new(mesh, |v: StrokeVertex<'_, '_>| {
            vertex(v.position().to_array(), color)
        }),
    );
}

fn vertex(position: [f32; 2], color: [u8; 3]) -> Vertex {
    Vertex {
        position,
        color: [color[0], color[1], color[2], 255],
    }
}

fn stroke_path(stroke: &Stroke) -> Path {
    let mut builder = Path::builder();
    let mut points = stroke.points.iter().map(|at| point(at[0], at[1]));
    if let Some(first) = points.next() {
        builder.begin(first);
        for at in points {
            builder.line_to(at);
        }
        builder.end(false);
    }
    builder.build()
}

fn arrow_path(arrow: &Arrow) -> Path {
    let [a, b, c, d] = arrow.p.map(|at| point(at[0], at[1]));
    let mut builder = Path::builder();
    builder.begin(a);
    builder.cubic_bezier_to(b, c, d);
    builder.end(false);
    builder.build()
}
