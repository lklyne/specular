// One render pass per window frame: a procedural dot grid under N textured,
// rounded page quads. Layouts mirror `gpu_types.rs`.

struct Frame {
    view_proj: mat4x4<f32>,
    viewport_px: vec2<f32>,
    scale_factor: f32,
    zoom: f32,
    grid_origin: vec2<f32>,
    grid_spacing: f32,
    dot_radius: f32,
    background: vec4<f32>,
    dot: vec4<f32>,
    encode_srgb: f32,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var page_texture: texture_2d<f32>;
@group(1) @binding(1) var page_sampler: sampler;

fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, linear <= vec3<f32>(0.0031308));
}

// Grid colours are linear; a non-sRGB target needs them encoded here.
fn to_target(color: vec4<f32>) -> vec4<f32> {
    if frame.encode_srgb > 0.5 {
        return vec4<f32>(linear_to_srgb(color.rgb), color.a);
    }
    return color;
}

@vertex
fn vs_grid(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    // Full-screen triangle.
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_grid(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    var color = frame.background;
    if frame.grid_spacing > 0.0 {
        let point = position.xy / frame.scale_factor - frame.grid_origin;
        let offset = point - frame.grid_spacing * round(point / frame.grid_spacing);
        let coverage = clamp(
            (frame.dot_radius - length(offset)) * frame.scale_factor + 0.5,
            0.0,
            1.0,
        );
        color = mix(frame.background, vec4<f32>(frame.dot.rgb, 1.0), coverage * frame.dot.a);
    }
    return to_target(color);
}

struct QuadOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // Logical screen px from the quad centre, and the quad's half extent.
    @location(1) local: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) radius: f32,
};

@vertex
fn vs_quad(
    @builtin(vertex_index) index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) corner_radius: f32,
) -> QuadOut {
    // Triangle strip: (0,0) (1,0) (0,1) (1,1).
    let corner = vec2<f32>(f32(index & 1u), f32((index >> 1u) & 1u));
    let world = rect.xy + corner * rect.zw;
    let size = rect.zw * frame.zoom;
    var out: QuadOut;
    out.position = frame.view_proj * vec4<f32>(world, 0.0, 1.0);
    out.uv = corner;
    out.local = (corner - 0.5) * size;
    out.half_size = size * 0.5;
    out.radius = min(corner_radius * frame.zoom, min(out.half_size.x, out.half_size.y));
    return out;
}

@fragment
fn fs_quad(in: QuadOut) -> @location(0) vec4<f32> {
    // Rounded-box signed distance, anti-aliased over one device pixel.
    let q = abs(in.local) - in.half_size + vec2<f32>(in.radius);
    let distance = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - in.radius;
    let coverage = clamp(0.5 - distance * frame.scale_factor, 0.0, 1.0);
    // Page texels are premultiplied, so scaling all channels clips the corner.
    return textureSample(page_texture, page_sampler, in.uv) * coverage;
}
