struct Frame {
    // screen = canvas * zoom + offset
    offset: vec2f,
    zoom: f32,
    viewport: vec2f,
}

@group(0) @binding(0) var<uniform> frame: Frame;

fn clip(screen: vec2f) -> vec4f {
    let ndc = screen / frame.viewport * 2.0 - 1.0;
    return vec4f(ndc.x, -ndc.y, 0.0, 1.0);
}

struct NoteVertex {
    @builtin(position) position: vec4f,
    @location(0) local: vec2f,
    @location(1) half_size: vec2f,
    @location(2) color: vec4f,
    @location(3) radius: f32,
}

// One instanced quad per rounded rect, grown by a pixel so the antialiased
// edge has room. Everything past this point is in screen pixels.
@vertex
fn note_vertex(
    @builtin(vertex_index) index: u32,
    @location(0) rect: vec4f,
    @location(1) color: vec4f,
    @location(2) radius: f32,
) -> NoteVertex {
    let corner = vec2f(f32(index & 1u), f32((index >> 1u) & 1u)) * 2.0 - 1.0;
    let half_size = rect.zw * 0.5 * frame.zoom;
    let center = (rect.xy + rect.zw * 0.5) * frame.zoom + frame.offset;
    let local = corner * (half_size + 1.0);

    var out: NoteVertex;
    out.position = clip(center + local);
    out.local = local;
    out.half_size = half_size;
    out.color = color;
    out.radius = min(radius * frame.zoom, min(half_size.x, half_size.y));
    return out;
}

@fragment
fn note_fragment(in: NoteVertex) -> @location(0) vec4f {
    let q = abs(in.local) - in.half_size + in.radius;
    let distance = length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - in.radius;
    return vec4f(in.color.rgb, in.color.a * clamp(0.5 - distance, 0.0, 1.0));
}

struct MeshVertex {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
}

// Tessellated strokes arrive in canvas units; MSAA does the antialiasing.
@vertex
fn mesh_vertex(@location(0) position: vec2f, @location(1) color: vec4f) -> MeshVertex {
    var out: MeshVertex;
    out.position = clip(position * frame.zoom + frame.offset);
    out.color = color;
    return out;
}

@fragment
fn mesh_fragment(in: MeshVertex) -> @location(0) vec4f {
    return in.color;
}
