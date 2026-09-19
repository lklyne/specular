// @ts-nocheck — WGSL source strings; no runtime types to check here, but the
// file sits next to rawWebGpuRenderer.ts which needs ts-nocheck for the same
// reason (no @webgpu/types in this repo).

/** Byte size of the `Uniforms` struct below: two vec4<f32> fields, 16-byte aligned. */
export const UNIFORM_STRUCT_SIZE = 32

/**
 * Shared across every pipeline: the dynamic-offset uniform binding (group 0),
 * the vertex shader that places a 4-vertex triangle-strip quad from a device-px
 * rect, and the rounded-rect alpha mask sampled in device-px space. Kept as one
 * string and prepended to each sampling-kind's fragment code so `vs_main` and
 * the mask math aren't duplicated by hand.
 */
const SHARED_WGSL = `
struct Uniforms {
  // x, y, w, h — destination rect in device px, within whatever target this
  // pass is rendering to (the canvas for a page draw, an owned texture for a
  // blit pass).
  rect: vec4<f32>,
  // targetWidth, targetHeight, cornerRadius, unused — device px.
  extra: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
  var corners = array<vec2<f32>, 4>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 1.0),
  );
  let corner = corners[vertexIndex];
  let px = u.rect.x + corner.x * u.rect.z;
  let py = u.rect.y + corner.y * u.rect.w;
  let clipX = (px / u.extra.x) * 2.0 - 1.0;
  let clipY = 1.0 - (py / u.extra.y) * 2.0;
  var out: VertexOutput;
  out.position = vec4<f32>(clipX, clipY, 0.0, 1.0);
  out.uv = corner;
  return out;
}

// Rounded-rect coverage from a box SDF, antialiased over ~1 device px.
fn roundedRectAlpha(uv: vec2<f32>, rectSize: vec2<f32>, radius: f32) -> f32 {
  let halfSize = rectSize * 0.5;
  let p = (uv - vec2<f32>(0.5, 0.5)) * rectSize;
  let q = abs(p) - halfSize + vec2<f32>(radius, radius);
  let d = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
  return 1.0 - smoothstep(-0.5, 0.5, d);
}
`

/**
 * Sampling kind for a live page frame: `texture_external`, imported fresh
 * every use (a GPUExternalTexture only lives for the task that created it).
 * `fs_mask` is the main-canvas draw (rounded, premultiplied, blended over);
 * `fs_plain` is the blit-into-owned-texture pass (no mask, opaque overwrite).
 */
export const EXTERNAL_TEXTURE_SHADER = `${SHARED_WGSL}
@group(1) @binding(0) var srcTex: texture_external;
@group(1) @binding(1) var srcSampler: sampler;

@fragment
fn fs_mask(in: VertexOutput) -> @location(0) vec4<f32> {
  let color = textureSampleBaseClampToEdge(srcTex, srcSampler, in.uv);
  let alpha = roundedRectAlpha(in.uv, u.rect.zw, u.extra.z) * color.a;
  return vec4<f32>(color.rgb * alpha, alpha);
}

@fragment
fn fs_plain(in: VertexOutput) -> @location(0) vec4<f32> {
  return textureSampleBaseClampToEdge(srcTex, srcSampler, in.uv);
}
`

/**
 * Sampling kind for an owned `rgba8unorm` texture (the blit arm's per-page
 * cache). Only the masked main-canvas draw is needed here — nothing samples
 * an owned texture without the rounded mask.
 */
export const TEXTURE_2D_SHADER = `${SHARED_WGSL}
@group(1) @binding(0) var srcTex: texture_2d<f32>;
@group(1) @binding(1) var srcSampler: sampler;

@fragment
fn fs_mask(in: VertexOutput) -> @location(0) vec4<f32> {
  let color = textureSample(srcTex, srcSampler, in.uv);
  let alpha = roundedRectAlpha(in.uv, u.rect.zw, u.extra.z) * color.a;
  return vec4<f32>(color.rgb * alpha, alpha);
}
`
