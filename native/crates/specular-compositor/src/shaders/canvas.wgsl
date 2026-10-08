// One render pass per window frame: a procedural dot grid under textured,
// rounded quads (pages and images), untextured SDF shapes and tessellated
// meshes. Layouts mirror `gpu_types.rs`.

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

// Scene colours are linear; a non-sRGB target needs them encoded here.
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
    @location(4) opacity: f32,
};

@vertex
fn vs_quad(
    @builtin(vertex_index) index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) uv_rect: vec4<f32>,
    @location(2) style: vec2<f32>,
) -> QuadOut {
    // Triangle strip: (0,0) (1,0) (0,1) (1,1).
    let corner = vec2<f32>(f32(index & 1u), f32((index >> 1u) & 1u));
    let world = rect.xy + corner * rect.zw;
    let size = rect.zw * frame.zoom;
    var out: QuadOut;
    out.position = frame.view_proj * vec4<f32>(world, 0.0, 1.0);
    out.uv = uv_rect.xy + corner * uv_rect.zw;
    out.local = (corner - 0.5) * size;
    out.half_size = size * 0.5;
    out.radius = min(style.x * frame.zoom, min(out.half_size.x, out.half_size.y));
    out.opacity = style.y;
    return out;
}

@fragment
fn fs_quad(in: QuadOut) -> @location(0) vec4<f32> {
    // Rounded-box signed distance, anti-aliased over one device pixel.
    let q = abs(in.local) - in.half_size + vec2<f32>(in.radius);
    let distance = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - in.radius;
    let coverage = clamp(0.5 - distance * frame.scale_factor, 0.0, 1.0);
    // Texels are premultiplied, so scaling all channels clips the corner and
    // fades the quad.
    return textureSample(page_texture, page_sampler, in.uv) * (coverage * in.opacity);
}

struct ShapeOut {
    @builtin(position) position: vec4<f32>,
    // Logical screen px from the shape centre.
    @location(0) local: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) fill: vec4<f32>,
    @location(3) stroke: vec4<f32>,
    // Corner radius, stroke width, stroke offset from the edge, kind.
    @location(4) style: vec4<f32>,
};

@vertex
fn vs_shape(
    @builtin(vertex_index) index: u32,
    @location(0) centre_and_half: vec4<f32>,
    @location(1) fill: vec4<f32>,
    @location(2) stroke: vec4<f32>,
    @location(3) style: vec4<f32>,
) -> ShapeOut {
    let corner = vec2<f32>(f32(index & 1u), f32((index >> 1u) & 1u));
    let half_size = centre_and_half.zw;
    // The quad reaches past the shape edge as far as the stroke does, or as
    // far as a shadow's blur still shows.
    var reach = max(style.z + style.y, 0.0);
    if style.w > 1.5 {
        reach = style.y * 1.5;
    }
    let local = (corner * 2.0 - 1.0) * (half_size + vec2<f32>(reach));
    let logical_viewport = frame.viewport_px / frame.scale_factor;
    var out: ShapeOut;
    out.position = frame.view_proj * vec4<f32>(centre_and_half.xy, 0.0, 1.0);
    out.position = vec4<f32>(
        out.position.xy + local * vec2<f32>(2.0, -2.0) / logical_viewport * out.position.w,
        out.position.zw,
    );
    out.local = local;
    out.half_size = half_size;
    out.fill = fill;
    out.stroke = stroke;
    out.style = style;
    return out;
}

// Signed distance to the shape edge in logical px: negative inside.
fn shape_distance(local: vec2<f32>, half_size: vec2<f32>, radius: f32, kind: f32) -> f32 {
    if kind > 0.5 && kind < 1.5 {
        // Ellipse, by the first-order estimate: exact for a circle, close
        // enough near the edge of any ellipse a canvas shape has.
        let axes = max(half_size, vec2<f32>(0.001));
        let k1 = length(local / axes);
        let k2 = length(local / (axes * axes));
        if k2 < 0.000001 {
            return -min(axes.x, axes.y);
        }
        return k1 * (k1 - 1.0) / k2;
    }
    let q = abs(local) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// The error function, to about 5e-4.
fn erf(x: f32) -> f32 {
    let a = abs(x);
    let t = 1.0 + a * (0.278393 + a * (0.230389 + a * (0.000972 + a * 0.078108)));
    let t2 = t * t;
    return sign(x) * (1.0 - 1.0 / (t2 * t2));
}

// How much of a shadow shows `distance` px outside its caster's edge: a
// straight edge under a Gaussian blur of the CSS radius `blur`, which is
// two standard deviations. Following the distance field instead of
// blurring rounds the corners a little more than a real blur would.
fn shadow_coverage(distance: f32, blur: f32) -> f32 {
    let sigma = max(blur * 0.5, 0.5 / frame.scale_factor);
    return 0.5 - 0.5 * erf(distance / (sigma * 1.41421356));
}

@fragment
fn fs_shape(in: ShapeOut) -> @location(0) vec4<f32> {
    let distance = shape_distance(in.local, in.half_size, in.style.x, in.style.w);
    if in.style.w > 1.5 {
        let shadow = to_target(in.fill);
        let alpha = shadow.a * shadow_coverage(distance, in.style.y);
        return vec4<f32>(shadow.rgb * alpha, alpha);
    }
    // The stroke is the band from `near` to `far`, measured outwards from
    // the edge: 0 to width sits outside it, -width to 0 inside.
    let near = in.style.z;
    let far = in.style.z + in.style.y;
    let scale = frame.scale_factor;
    let inside = clamp(0.5 - distance * scale, 0.0, 1.0);
    let band = max(
        clamp(0.5 - (distance - far) * scale, 0.0, 1.0)
            - clamp(0.5 - (distance - near) * scale, 0.0, 1.0),
        0.0,
    );
    // The part of the fill the stroke lies over.
    let covered = max(
        clamp(0.5 - (distance - min(far, 0.0)) * scale, 0.0, 1.0)
            - clamp(0.5 - (distance - min(near, 0.0)) * scale, 0.0, 1.0),
        0.0,
    );
    let fill = to_target(in.fill);
    let stroke = to_target(in.stroke);
    let stroke_alpha = stroke.a * band;
    // The fill shows whole where no stroke lies over it and through the
    // stroke where one does, so the premultiplied contributions add.
    let fill_alpha = fill.a * (max(inside - covered, 0.0) + covered * (1.0 - stroke.a));
    return vec4<f32>(
        fill.rgb * fill_alpha + stroke.rgb * stroke_alpha,
        fill_alpha + stroke_alpha,
    );
}

struct MeshOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Mesh vertices are already in logical screen px.
@vertex
fn vs_mesh(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>) -> MeshOut {
    let logical_viewport = frame.viewport_px / frame.scale_factor;
    var out: MeshOut;
    out.position = vec4<f32>(
        position / logical_viewport * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0),
        0.0,
        1.0,
    );
    out.color = color;
    return out;
}

@fragment
fn fs_mesh(in: MeshOut) -> @location(0) vec4<f32> {
    let color = to_target(in.color);
    return vec4<f32>(color.rgb * color.a, color.a);
}

// For the multiply pipeline, whose blend is `target * this`: the colour at
// full alpha, white (no change) at none.
@fragment
fn fs_mesh_multiply(in: MeshOut) -> @location(0) vec4<f32> {
    let color = to_target(in.color);
    return vec4<f32>(mix(vec3<f32>(1.0), color.rgb, color.a), 1.0);
}
