//! The two meshes, generated as the web build generates them
//! (`scene/rounded-geometry.ts`), in the same vertex and index order.

/// The shell: a cube subdivided `segments` times a side and pushed onto the
/// superellipsoid `|x|^n + |y|^n + |z|^n = 1`, with its exact normals.
#[derive(Debug, Clone)]
pub(crate) struct ShellMesh {
    pub(crate) positions: Vec<[f32; 3]>,
    pub(crate) normals: Vec<[f32; 3]>,
    pub(crate) indices: Vec<u32>,
}

/// The card: a flat superellipse in XY, with UVs 0..1 across its box.
#[derive(Debug, Clone)]
pub(crate) struct CardMesh {
    pub(crate) uvs: Vec<[f32; 2]>,
    pub(crate) positions: Vec<[f32; 3]>,
    pub(crate) indices: Vec<u32>,
}

/// Axis order of one face of three's `BoxGeometry`: (u, v, w, udir, vdir,
/// sign of w), in its face order +x, -x, +y, -y, +z, -z.
const FACES: [(usize, usize, usize, f64, f64, f64); 6] = [
    (2, 1, 0, -1.0, -1.0, 1.0),
    (2, 1, 0, 1.0, -1.0, -1.0),
    (0, 2, 1, 1.0, 1.0, 1.0),
    (0, 2, 1, 1.0, -1.0, -1.0),
    (0, 1, 2, 1.0, -1.0, 1.0),
    (0, 1, 2, -1.0, -1.0, -1.0),
];

/// `createRoundedBoxGeometry(size, squircleness, segments)`.
#[expect(
    clippy::many_single_char_names,
    reason = "the names of three's BoxGeometry.buildPlane"
)]
pub(crate) fn shell(size: f64, squircleness: f64, segments: u32) -> ShellMesh {
    let half = size / 2.0;
    let n = squircleness.max(2.0);
    let seg = segments.max(1);
    let row = seg + 1;
    let mut positions = Vec::with_capacity((6 * row * row) as usize);
    let mut normals = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity((36 * seg * seg) as usize);
    let step = size / f64::from(seg);
    for (u, v, w, udir, vdir, wsign) in FACES {
        let base = positions.len() as u32;
        for iy in 0..row {
            let y = f64::from(iy) * step - half;
            for ix in 0..row {
                let x = f64::from(ix) * step - half;
                let mut p = [0.0_f64; 3];
                p[u] = x * udir;
                p[v] = y * vdir;
                p[w] = half * wsign;
                // three reads the box back out of a Float32 buffer.
                let p = p.map(|c| f64::from(c as f32) / half);
                let scale = p.iter().map(|c| c.abs().powf(n)).sum::<f64>().powf(1.0 / n);
                let scale = if scale == 0.0 { 1.0 } else { scale };
                let p = p.map(|c| c / scale);
                let g = p.map(|c| c.signum() * c.abs().powf(n - 1.0));
                let len = g.iter().map(|c| c * c).sum::<f64>().sqrt();
                let len = if len == 0.0 { 1.0 } else { len };
                positions.push(p.map(|c| (c * half) as f32));
                normals.push(g.map(|c| (c / len) as f32));
            }
        }
        for iy in 0..seg {
            for ix in 0..seg {
                let a = base + ix + row * iy;
                let b = base + ix + row * (iy + 1);
                let c = base + (ix + 1) + row * (iy + 1);
                let d = base + (ix + 1) + row * iy;
                indices.extend([a, b, d, b, c, d]);
            }
        }
    }
    ShellMesh {
        positions,
        normals,
        indices,
    }
}

/// `createSquircleGeometry(size, size, squircleness, segments)`. The
/// outline is convex, so a fan covers it as three's earcut does.
pub(crate) fn card(size: f64, squircleness: f64, segments: u32) -> CardMesh {
    let a = size / 2.0;
    let exponent = 2.0 / squircleness.max(2.0);
    let seg = segments.max(8);
    let signed_pow = |v: f64| v.signum() * v.abs().powf(exponent);
    let mut uvs = Vec::with_capacity(seg as usize);
    let mut positions = Vec::with_capacity(seg as usize);
    for i in 0..seg {
        let theta = f64::from(i) / f64::from(seg) * std::f64::consts::TAU;
        let x = (a * signed_pow(theta.cos())) as f32;
        let y = (a * signed_pow(theta.sin())) as f32;
        positions.push([x, y, 0.0]);
        uvs.push([
            ((f64::from(x) + a) / size) as f32,
            ((f64::from(y) + a) / size) as f32,
        ]);
    }
    // three's ShapeGeometry stores the outline the other way round.
    uvs.reverse();
    positions.reverse();
    let indices = (1..seg - 1).flat_map(|i| [0, i, i + 1]).collect();
    CardMesh {
        uvs,
        positions,
        indices,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn floats(path: &Path) -> Option<Vec<f32>> {
        std::fs::read(path)
            .ok()
            .map(|b| bytemuck::pod_collect_to_vec(&b))
    }

    /// Against the web build's own buffers, when the capture is on disk.
    #[test]
    fn meshes_match_the_captured_buffers() {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.prism-work/reference/geometry");
        let Some(positions) = floats(&dir.join("02-ShellGlass-slot0.f32")) else {
            return;
        };
        let shell = shell(1.2, 4.0, 80);
        let normals = floats(&dir.join("02-ShellGlass-slot1.f32")).unwrap();
        let indices: Vec<u32> = bytemuck::pod_collect_to_vec(
            &std::fs::read(dir.join("02-ShellGlass-index.uint32")).unwrap(),
        );
        assert_eq!(shell.indices, indices);
        let flat = |v: &[[f32; 3]]| v.iter().flatten().copied().collect::<Vec<_>>();
        let worst = |a: &[f32], b: &[f32]| {
            a.iter()
                .zip(b)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0, f32::max)
        };
        assert_eq!(positions.len(), shell.positions.len() * 3);
        assert!(worst(&positions, &flat(&shell.positions)) < 1e-6);
        assert!(worst(&normals, &flat(&shell.normals)) < 1e-6);

        let card = card(0.5928, 6.0, 256);
        let positions = floats(&dir.join("00-MeshBasicNodeMaterial-slot1.f32")).unwrap();
        let uvs = floats(&dir.join("00-MeshBasicNodeMaterial-slot0.f32")).unwrap();
        assert_eq!(card.indices.len(), 762);
        assert!(worst(&positions, &flat(&card.positions)) < 1e-6);
        let uv_flat: Vec<f32> = card.uvs.iter().flatten().copied().collect();
        assert!(worst(&uvs, &uv_flat) < 1e-6);
    }
}
