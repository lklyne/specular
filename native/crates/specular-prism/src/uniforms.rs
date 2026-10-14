//! Fills each pass's uniform blocks from [`PrismParams`] and the time.
//!
//! The member names are three's (`nodeUniformN`); this file is the one
//! place that knows which is which. `reference/passes.md` lists the same
//! mapping as the capture decoded it.

use crate::block::{Block, Value};
use crate::params::{PrismParams, Shell};

/// Half the camera's view, in world units, at any canvas size.
const HALF_EXTENT: f32 = 0.7;
const NEAR: f32 = 0.1;
const FAR: f32 = 50.0;
const EYE_Z: f32 = 5.0;

/// The baked PMREM atlas: its highest mip and its texel size.
const PMREM_MAX_MIP: f32 = 6.0;
const PMREM_SIZE: (f32, f32) = (336.0, 256.0);

/// The painted-normal effect is wired into the glass but off for Prism.
const BRUSH_WORLD_SCALE: f32 = 3.0;
const BRUSH_BLEND_SHARPNESS: f32 = 4.0;
const BRUSH_STRENGTH: f32 = 0.0;
const BRUSH_SPECULAR_AA: f32 = 1.0;

const IDENTITY4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Inputs shared by every pass of one frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Frame {
    /// Seconds since the scene started.
    pub(crate) t: f32,
    /// Square render size in pixels.
    pub(crate) size: f32,
}

fn projection() -> [[f32; 4]; 4] {
    let depth = FAR - NEAR;
    [
        [1.0 / HALF_EXTENT, 0.0, 0.0, 0.0],
        [0.0, 1.0 / HALF_EXTENT, 0.0, 0.0],
        [0.0, 0.0, -1.0 / depth, 0.0],
        [0.0, 0.0, -NEAR / depth, 1.0],
    ]
}

fn translation(z: f32) -> [[f32; 4]; 4] {
    let mut m = IDENTITY4;
    m[3][2] = z;
    m
}

fn scaling(s: f32) -> [[f32; 4]; 4] {
    let mut m = IDENTITY4;
    for (i, column) in m.iter_mut().take(3).enumerate() {
        column[i] = s;
    }
    m
}

fn normal_matrix(scale: f32) -> [[f32; 3]; 3] {
    let s = 1.0 / scale;
    [[s, 0.0, 0.0], [0.0, s, 0.0], [0.0, 0.0, s]]
}

fn env_rotation(angle: f32) -> [[f32; 4]; 4] {
    let (s, c) = angle.sin_cos();
    [
        [c, 0.0, s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-s, 0.0, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Group 0 of the card.
pub(crate) fn card_render(block: &mut Block, p: &PrismParams) {
    block.set("cameraProjectionMatrix", Value::Mat4(projection()));
    block.set("cameraViewMatrix", Value::Mat4(translation(-EYE_Z)));
    block.set("nodeUniform12", Value::Vec3(p.light.ambient));
}

/// Group 1 of the card: the gradient.
#[expect(
    clippy::many_single_char_names,
    reason = "short names keep the uniform table readable"
)]
pub(crate) fn card_object(block: &mut Block, colors: &mut Block, p: &PrismParams, f: Frame) {
    let g = &p.gradient;
    block.set("nodeUniform0", Value::F32(g.distortion));
    block.set("nodeUniform1", Value::F32(p.gradient_time(f.t)));
    block.set("nodeUniform2", Value::F32(g.swirl));
    block.set("nodeUniform3", Value::F32(g.colors.len().min(8) as f32));
    block.set("nodeUniform5", Value::F32(g.falloff));
    block.set("nodeUniform6", Value::F32(g.speckle_size));
    block.set("nodeUniform7", Value::F32(g.grain_seed));
    block.set("nodeUniform8", Value::F32(g.speckle_amount));
    block.set("nodeUniform9", Value::F32(g.posterize_levels));
    block.set("nodeUniform10", Value::F32(g.posterize_amount));
    block.set("nodeUniform11", Value::F32(1.0));
    block.set("nodeUniform13", Value::Mat4(IDENTITY4));
    // Unused slots repeat the last colour, as three pads the array.
    let last = g.colors.last().copied().unwrap_or_default();
    let padded: Vec<[f32; 4]> = (0..8)
        .map(|i| {
            let [r, gr, b] = g.colors.get(i).copied().unwrap_or(last);
            [r, gr, b, 0.0]
        })
        .collect();
    colors.set("value", Value::Vec4s(&padded));
}

/// The camera members both shells' group 0 share.
fn shell_camera(block: &mut Block) {
    block.set("cameraProjectionMatrix", Value::Mat4(projection()));
    block.set("cameraViewMatrix", Value::Mat4(translation(-EYE_Z)));
    block.set("cameraWorldMatrix", Value::Mat4(translation(EYE_Z)));
    block.set("cameraPosition", Value::Vec3([0.0, 0.0, EYE_Z]));
}

/// Group 0 of the liquid.
pub(crate) fn liquid_render(block: &mut Block, p: &PrismParams, f: Frame) {
    shell_camera(block);
    block.set("nodeUniform20", Value::Vec3(p.light.color));
    block.set("nodeUniform21", Value::Vec3(p.light.ambient));
    block.set("nodeUniform18", Value::Vec3(p.light_position(f.t)));
    block.set("nodeUniform19", Value::Vec3([0.0; 3]));
    block.set("nodeUniform45", Value::Vec2([f.size; 2]));
    block.set("nodeUniform40", Value::Vec4([0.0, 0.0, f.size, f.size]));
}

/// Group 0 of the glass.
pub(crate) fn glass_render(block: &mut Block, p: &PrismParams, f: Frame) {
    shell_camera(block);
    let light = p.light_position(f.t);
    block.set("nodeUniform23", Value::F32(f.t));
    block.set("nodeUniform34", Value::Vec3(p.light.color));
    block.set("nodeUniform35", Value::Vec3(p.light.ambient));
    block.set("nodeUniform32", Value::Vec3(light));
    block.set("nodeUniform33", Value::Vec3([0.0; 3]));
    block.set(
        "nodeUniform28",
        Value::Vec3([light[0], light[1], light[2] - EYE_Z]),
    );
    block.set("nodeUniform58", Value::Vec2([f.size; 2]));
    block.set("nodeUniform53", Value::Vec4([0.0, 0.0, f.size, f.size]));
}

/// The environment members, named per shader: (max mip, rotation, texel
/// width, texel height, intensity).
fn environment(block: &mut Block, p: &PrismParams, names: [&str; 5]) {
    block.set(names[0], Value::F32(PMREM_MAX_MIP));
    block.set(names[1], Value::Mat4(env_rotation(p.environment.rotation)));
    block.set(names[2], Value::F32(1.0 / PMREM_SIZE.0));
    block.set(names[3], Value::F32(1.0 / PMREM_SIZE.1));
    block.set(names[4], Value::F32(p.environment.intensity));
}

/// Group 1 of the liquid.
pub(crate) fn liquid_object(block: &mut Block, p: &PrismParams, f: Frame) {
    let s: &Shell = &p.liquid;
    let world = Value::Mat4(scaling(s.scale));
    block.set("nodeUniform0", Value::Vec3(s.color));
    block.set("nodeUniform1", Value::F32(1.0));
    block.set("nodeUniform2", Value::F32(0.0));
    block.set("nodeUniform3", Value::F32(s.roughness));
    block.set("nodeUniform5", Value::Mat3(normal_matrix(s.scale)));
    block.set("nodeUniform6", Value::F32(s.ior));
    block.set("nodeUniform7", Value::Vec3(s.specular_color));
    block.set("nodeUniform8", Value::F32(s.specular_intensity));
    block.set("nodeUniform9", Value::Vec3([0.0; 3]));
    block.set("nodeUniform10", Value::F32(1.0));
    block.set("nodeUniform12", world);
    block.set("nodeUniform13", Value::F32(s.distortion_scale));
    block.set("nodeUniform14", Value::F32(s.surface_distortion));
    block.set("nodeUniform16", Value::F32(f.t));
    block.set("nodeUniform17", Value::F32(s.temporal_distortion));
    environment(
        block,
        p,
        [
            "nodeUniform22",
            "nodeUniform23",
            "nodeUniform25",
            "nodeUniform26",
            "nodeUniform28",
        ],
    );
    block.set("nodeUniform29", Value::F32(s.backdrop_distance));
    block.set("nodeUniform30", Value::F32(s.anisotropic_blur));
    block.set("nodeUniform32", world);
    block.set("nodeUniform33", Value::F32(s.thickness));
    block.set("nodeUniform34", Value::F32(s.distortion_scale));
    block.set("nodeUniform35", Value::F32(s.distortion));
    block.set("nodeUniform36", Value::F32(f.t));
    block.set("nodeUniform37", Value::F32(s.temporal_distortion));
    block.set("nodeUniform38", Value::F32(s.dither_scale));
    block.set("nodeUniform39", Value::F32(s.dither_strength));
    block.set("nodeUniform41", Value::F32(s.edge_fade));
    block.set("nodeUniform42", Value::F32(s.dispersion));
    block.set("nodeUniform43", Value::F32(s.chromatic_aberration));
    block.set("nodeUniform47", Value::F32(s.blur_scale));
    block.set("nodeUniform48", Value::Vec3(s.backdrop_fallback_color));
    block.set("nodeUniform49", Value::F32(s.backdrop_fallback));
    block.set("nodeUniform50", Value::F32(s.attenuation_distance));
    block.set("nodeUniform51", Value::Vec3(s.attenuation_color));
    block.set("nodeUniform52", Value::F32(s.transmission));
}

/// Group 1 of the glass.
pub(crate) fn glass_object(block: &mut Block, p: &PrismParams, f: Frame) {
    let s: &Shell = &p.glass;
    let world = Value::Mat4(scaling(s.scale));
    block.set("nodeUniform0", Value::Vec3(s.color));
    block.set("nodeUniform1", Value::F32(1.0));
    block.set("nodeUniform2", Value::F32(0.0));
    block.set("nodeUniform3", Value::F32(s.roughness));
    block.set("nodeUniform4", world);
    block.set("nodeUniform5", Value::F32(BRUSH_WORLD_SCALE));
    block.set("nodeUniform7", Value::Mat3(normal_matrix(s.scale)));
    block.set("nodeUniform9", Value::F32(BRUSH_BLEND_SHARPNESS));
    block.set("nodeUniform11", Value::F32(BRUSH_STRENGTH));
    block.set("nodeUniform12", Value::F32(s.distortion_scale));
    block.set("nodeUniform13", Value::F32(s.surface_distortion));
    block.set("nodeUniform14", Value::F32(f.t));
    block.set("nodeUniform15", Value::F32(s.temporal_distortion));
    block.set("nodeUniform16", Value::F32(BRUSH_SPECULAR_AA));
    block.set("nodeUniform17", Value::F32(s.ior));
    block.set("nodeUniform18", Value::Vec3(s.specular_color));
    block.set("nodeUniform19", Value::F32(s.specular_intensity));
    block.set("nodeUniform20", Value::Vec3(p.rim.color));
    block.set("nodeUniform21", Value::F32(p.rim.power));
    block.set("nodeUniform22", Value::F32(p.rim.intensity));
    block.set("nodeUniform24", Value::F32(p.rim.pulse));
    block.set("nodeUniform25", Value::Vec3(p.highlight.color));
    block.set("nodeUniform26", Value::F32(p.highlight.threshold));
    block.set("nodeUniform27", Value::F32(p.highlight.softness));
    block.set("nodeUniform29", Value::F32(p.highlight.shininess));
    block.set("nodeUniform30", Value::F32(p.highlight.intensity));
    environment(
        block,
        p,
        [
            "nodeUniform36",
            "nodeUniform37",
            "nodeUniform39",
            "nodeUniform40",
            "nodeUniform42",
        ],
    );
    block.set("nodeUniform43", Value::F32(s.backdrop_distance));
    block.set("nodeUniform44", Value::F32(s.anisotropic_blur));
    block.set("nodeUniform46", world);
    block.set("nodeUniform47", Value::F32(s.thickness));
    block.set("nodeUniform48", Value::F32(s.distortion_scale));
    block.set("nodeUniform49", Value::F32(s.distortion));
    block.set("nodeUniform50", Value::F32(f.t));
    block.set("nodeUniform51", Value::F32(s.temporal_distortion));
    block.set("nodeUniform52", Value::F32(s.dither_strength));
    block.set("nodeUniform54", Value::F32(s.edge_fade));
    block.set("nodeUniform55", Value::F32(s.dispersion));
    block.set("nodeUniform56", Value::F32(s.chromatic_aberration));
    block.set("nodeUniform60", Value::F32(s.blur_scale));
    block.set("nodeUniform61", Value::Vec3(s.backdrop_fallback_color));
    block.set("nodeUniform62", Value::F32(s.backdrop_fallback));
    block.set("nodeUniform63", Value::F32(s.attenuation_distance));
    block.set("nodeUniform64", Value::Vec3(s.attenuation_color));
    block.set("nodeUniform65", Value::F32(s.transmission));
}

/// The display pass: the grade, then the paper.
pub(crate) fn display(block: &mut Block, p: &PrismParams) {
    let g = &p.grade;
    block.set(
        "nodeUniform1",
        Value::Mat3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
    );
    block.set("nodeUniform2", Value::F32(g.exposure));
    block.set("nodeUniform3", Value::F32(g.shadow_lift));
    block.set("nodeUniform4", Value::F32(g.gamma));
    block.set("nodeUniform5", Value::F32(g.contrast));
    block.set("nodeUniform6", Value::F32(g.saturation));
    block.set("nodeUniform7", Value::F32(g.vibrance));
    block.set("nodeUniform8", Value::F32(g.temperature));
    block.set("nodeUniform9", Value::F32(g.tint));
    block.set("nodeUniform10", Value::F32(g.amount));
    block.set("nodeUniform11", Value::Vec3(g.paper));
}
