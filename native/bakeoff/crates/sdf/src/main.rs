//! Candidate B: SDF rounded rects, glyphon text and lyon-tessellated strokes,
//! each its own pipeline inside one 4x MSAA pass.
//!
//! `--retained` keeps the stroke mesh for the whole world while the zoom
//! holds; the default tessellates the visible strokes every frame. `--lod`
//! skips text too small to read.

mod gpu_parts;
mod mesh;

use anyhow::Result;
use bakeoff_scene::{
    Camera, Gpu, World,
    camera::VIEWPORT,
    harness::{self, Candidate},
    world,
};
use bakeoff_ui::Panels;
use glyphon::{
    Attrs, Buffer, Cache, Color, ColorMode, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use gpu_parts::{FORMAT, FrameUniform, GrowBuffer, NoteInstance, Pipelines, SAMPLES};
use mesh::{Mesh, Mesher};

struct Text {
    fonts: FontSystem,
    swash: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    /// One shaped, wrapped buffer per note, in canvas units.
    buffers: Vec<Buffer>,
    lod: bool,
}

struct SdfCandidate {
    pipelines: Pipelines,
    multisampled: wgpu::TextureView,
    target: wgpu::Texture,
    resolve: wgpu::TextureView,
    text: Text,
    mesher: Mesher,
    mesh: Mesh,
    /// Zoom the retained mesh was tessellated for, when `--retained`.
    retained_zoom: Option<f32>,
    retained: bool,
    instances: Vec<NoteInstance>,
    instance_buffer: GrowBuffer,
    vertex_buffer: GrowBuffer,
    index_buffer: GrowBuffer,
}

impl SdfCandidate {
    fn new(gpu: &Gpu, world: &World, retained: bool) -> Self {
        let attachment = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let target = gpu.target(FORMAT, attachment | wgpu::TextureUsages::COPY_SRC, 1);
        Self {
            pipelines: Pipelines::new(gpu),
            multisampled: gpu
                .target(FORMAT, attachment, SAMPLES)
                .create_view(&wgpu::TextureViewDescriptor::default()),
            resolve: target.create_view(&wgpu::TextureViewDescriptor::default()),
            target,
            text: Text::new(gpu, world),
            mesher: Mesher::new(world),
            mesh: Mesh::new(),
            retained_zoom: None,
            retained,
            instances: Vec::new(),
            instance_buffer: GrowBuffer::new(gpu, wgpu::BufferUsages::VERTEX),
            vertex_buffer: GrowBuffer::new(gpu, wgpu::BufferUsages::VERTEX),
            index_buffer: GrowBuffer::new(gpu, wgpu::BufferUsages::INDEX),
        }
    }

    fn upload(&mut self, gpu: &Gpu, world: &World, camera: Camera) {
        let frame = FrameUniform {
            offset: camera.offset(),
            zoom: camera.zoom,
            _pad: 0.0,
            viewport: VIEWPORT.map(|side| side as f32),
            _pad2: [0.0; 2],
        };
        gpu.queue
            .write_buffer(&self.pipelines.frame, 0, bytemuck::bytes_of(&frame));

        self.instances.clear();
        self.instances.extend(
            world
                .notes
                .iter()
                .filter(|note| camera.sees(note.origin, world::NOTE_SIZE))
                .map(|note| NoteInstance {
                    rect: [
                        note.origin[0],
                        note.origin[1],
                        world::NOTE_SIZE[0],
                        world::NOTE_SIZE[1],
                    ],
                    color: [note.color[0], note.color[1], note.color[2], 255],
                    radius: world::NOTE_RADIUS,
                }),
        );
        self.instance_buffer.write(gpu, &self.instances);

        if !self.retained {
            self.mesher
                .tessellate(&mut self.mesh, world, camera.zoom, Some(camera));
        } else if self.retained_zoom != Some(camera.zoom) {
            self.mesher
                .tessellate(&mut self.mesh, world, camera.zoom, None);
            self.retained_zoom = Some(camera.zoom);
        } else {
            return;
        }
        self.vertex_buffer.write(gpu, &self.mesh.vertices);
        self.index_buffer.write(gpu, &self.mesh.indices);
    }
}

impl Candidate for SdfCandidate {
    fn frame(&mut self, gpu: &Gpu, world: &World, camera: Camera) -> Result<()> {
        self.upload(gpu, world, camera);
        self.text.prepare(gpu, world, camera)?;

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let [r, g, b] = world::BACKGROUND.map(|channel| f64::from(channel) / 255.0);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("canvas"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisampled,
                    depth_slice: None,
                    resolve_target: Some(&self.resolve),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                ..wgpu::RenderPassDescriptor::default()
            });
            pass.set_bind_group(0, &self.pipelines.frame_bind, &[]);

            pass.set_pipeline(&self.pipelines.notes);
            pass.set_vertex_buffer(0, self.instance_buffer.buffer.slice(..));
            pass.draw(0..4, 0..self.instances.len() as u32);

            self.text
                .renderer
                .render(&self.text.atlas, &self.text.viewport, &mut pass)?;

            pass.set_bind_group(0, &self.pipelines.frame_bind, &[]);
            pass.set_pipeline(&self.pipelines.mesh);
            pass.set_vertex_buffer(0, self.vertex_buffer.buffer.slice(..));
            pass.set_index_buffer(
                self.index_buffer.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.mesh.indices.len() as u32, 0, 0..1);
        }
        gpu.queue.submit([encoder.finish()]);
        self.text.atlas.trim();
        Ok(())
    }

    fn target(&self) -> &wgpu::Texture {
        &self.target
    }
}

impl Text {
    fn new(gpu: &Gpu, world: &World) -> Self {
        let mut fonts = FontSystem::new();
        let cache = Cache::new(&gpu.device);
        // The target is not sRGB, so colours must go out as written.
        let mut atlas =
            TextAtlas::with_color_mode(&gpu.device, &gpu.queue, &cache, FORMAT, ColorMode::Web);
        let multisample = wgpu::MultisampleState {
            count: SAMPLES,
            ..Default::default()
        };
        let renderer = TextRenderer::new(&mut atlas, &gpu.device, multisample, None);

        let width = world::NOTE_SIZE[0] - 2.0 * world::NOTE_PADDING;
        let attrs = Attrs::new().family(Family::Name(world::FONT_FAMILY));
        let buffers = world
            .notes
            .iter()
            .map(|note| {
                let metrics = Metrics::new(world::FONT_SIZE, world::LINE_HEIGHT);
                let mut buffer = Buffer::new(&mut fonts, metrics);
                buffer.set_size(Some(width), None);
                buffer.set_text(&note.text, &attrs, Shaping::Advanced, None);
                buffer.shape_until_scroll(&mut fonts, false);
                buffer
            })
            .collect();

        Self {
            viewport: Viewport::new(&gpu.device, &cache),
            fonts,
            swash: SwashCache::new(),
            atlas,
            renderer,
            buffers,
            lod: harness::flag("--lod"),
        }
    }

    /// Rasterises any glyph not yet in the atlas at this zoom and lays out the
    /// quads. Layout stays in canvas units; glyphon scales it by `zoom`.
    fn prepare(&mut self, gpu: &Gpu, world: &World, camera: Camera) -> Result<()> {
        let [width, height] = VIEWPORT;
        self.viewport
            .update(&gpu.queue, Resolution { width, height });
        let [r, g, b] = world::TEXT_COLOR;
        let shown = camera.shows_text(self.lod);
        let areas = world
            .notes
            .iter()
            .zip(&self.buffers)
            .filter(|(note, _)| shown && camera.sees(note.origin, world::NOTE_SIZE))
            .map(|(note, buffer)| {
                let [left, top] = camera.to_screen([
                    note.origin[0] + world::NOTE_PADDING,
                    note.origin[1] + world::NOTE_PADDING,
                ]);
                TextArea {
                    buffer,
                    left,
                    top,
                    scale: camera.zoom,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: width as i32,
                        bottom: height as i32,
                    },
                    default_color: Color::rgb(r, g, b),
                    custom_glyphs: &[],
                }
            });
        self.renderer.prepare(
            &gpu.device,
            &gpu.queue,
            &mut self.fonts,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash,
        )?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let retained = harness::flag("--retained");
    let name = &harness::run_name("sdf");
    let gpu = Gpu::headless()?;
    let world = World::build();
    let mut candidate = SdfCandidate::new(&gpu, &world, retained);

    harness::golden_images(name, &gpu, &world, &mut candidate)?;
    let mut panels = Panels::new(&gpu, FORMAT);
    harness::panel_overlay(name, &gpu, &world, &mut candidate, |gpu, view, zoom| {
        panels.paint(gpu, view, zoom);
    })?;
    harness::timings(name, &gpu, &world, &mut candidate)
}
