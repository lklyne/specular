//! The renderer: page frames + dot grid under a camera, one pass per frame.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use specular_core::{
    CpuFrame, FrameEvent, FrameLayer, NativeSurface, PageEvent, PageFrame, PageId, PixelSize,
    SharedTexture,
};

use crate::draw_list::{DrawItem, LayerKind, build_draw_list};
use crate::error::CompositorError;
use crate::gpu_types::{FRAME_UNIFORMS_SIZE, FrameUniforms, QuadInstance};
use crate::grid::grid_metrics;
use crate::import::import_shared;
use crate::import_cache::ImportCache;
use crate::layers::{LayerTexture, PageLayers};
use crate::pipeline::Pipelines;
use crate::retire::RetiredTextures;
use crate::scene::{RenderStats, SceneView};
use crate::upload;

/// Instance capacity of the first instance buffer; it doubles on demand.
const INITIAL_INSTANCE_CAPACITY: usize = 64;

/// Identity of an imported shared surface: the same surface at the same
/// size and format imports to the same texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SurfaceKey {
    surface: NativeSurface,
    size: PixelSize,
    format: wgpu::TextureFormat,
}

/// A shared surface wrapped for sampling.
#[derive(Debug)]
struct ImportedTexture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

/// Frame counters accumulated between two renders.
#[derive(Debug, Clone, Copy, Default)]
struct IngestCounts {
    frames_received: u32,
    popup_frames: u32,
    frames_dropped_for_pool_pressure: u32,
}

/// The wgpu compositor; see the crate docs.
#[derive(Debug)]
pub struct Compositor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target_format: wgpu::TextureFormat,
    pipelines: Pipelines,
    frame_uniforms: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    pages: HashMap<PageId, PageLayers>,
    retired: RetiredTextures<SharedTexture>,
    imports: HashMap<(PageId, LayerKind), ImportCache<SurfaceKey, ImportedTexture>>,
    /// Serial of the most recent submit; 0 before the first.
    submitted: u64,
    /// Highest serial the GPU has reported complete.
    completed: Arc<AtomicU64>,
    ingested: IngestCounts,
    instances: Vec<QuadInstance>,
    draw_items: Vec<DrawItem>,
}

impl Compositor {
    /// Creates a compositor that renders into textures of `target_format`
    /// (the window surface's format).
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let pipelines = Pipelines::new(&device, target_format);
        let frame_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame-uniforms"),
            size: FRAME_UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame-bind-group"),
            layout: &pipelines.frame_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_uniforms.as_entire_binding(),
            }],
        });
        let instance_buffer = create_instance_buffer(&device, INITIAL_INSTANCE_CAPACITY);
        Self {
            device,
            queue,
            target_format,
            pipelines,
            frame_uniforms,
            frame_bind_group,
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            pages: HashMap::new(),
            retired: RetiredTextures::default(),
            imports: HashMap::new(),
            submitted: 0,
            completed: Arc::new(AtomicU64::new(0)),
            ingested: IngestCounts::default(),
            instances: Vec::with_capacity(INITIAL_INSTANCE_CAPACITY),
            draw_items: Vec::with_capacity(INITIAL_INSTANCE_CAPACITY),
        }
    }

    /// The colour format this compositor renders into.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// Ingests one event from a page source: frames replace the page's
    /// texture for that layer (the previous shared surface is released once
    /// the GPU is done with it); popup events show/hide/move the popup layer;
    /// dropped-frame events are counted; others are ignored.
    ///
    /// The per-page cap on shared surfaces
    /// ([`MAX_OUTSTANDING_TEXTURES`](specular_core::MAX_OUTSTANDING_TEXTURES))
    /// is the source's to enforce; the compositor reports what it holds.
    pub fn handle_page_event(&mut self, event: PageEvent) -> Result<(), CompositorError> {
        match event {
            PageEvent::Frame(frame) => self.ingest_frame(frame),
            PageEvent::FrameDropped { .. } => {
                self.ingested.frames_dropped_for_pool_pressure += 1;
                Ok(())
            }
            PageEvent::PopupVisibility { page, visible } => {
                let layers = self.pages.entry(page).or_default();
                layers.popup.visible = visible;
                if !visible {
                    let old = layers.popup.texture.take();
                    self.retire(page, old);
                }
                Ok(())
            }
            PageEvent::PopupRect { page, rect } => {
                self.pages.entry(page).or_default().popup.rect = Some(rect);
                Ok(())
            }
            PageEvent::ImeCompositionBounds { .. }
            | PageEvent::Loaded { .. }
            | PageEvent::Crashed { .. } => Ok(()),
        }
    }

    /// Forgets a closed page; its shared surfaces are released once the GPU
    /// has finished with them.
    pub fn remove_page(&mut self, page: PageId) {
        if let Some(layers) = self.pages.remove(&page) {
            self.retire(page, layers.view);
            self.retire(page, layers.popup.texture);
        }
        self.imports.remove(&(page, LayerKind::View));
        self.imports.remove(&(page, LayerKind::Popup));
    }

    /// Shared-surface imports served from the per-layer cache, and imports
    /// made, since startup. A steady page should almost never miss; a miss
    /// rate near 1 means the producer is not recycling its surfaces.
    pub fn import_cache_hits_and_misses(&self) -> (u64, u64) {
        self.imports
            .values()
            .map(ImportCache::hits_and_misses)
            .fold((0, 0), |(h, m), (hits, misses)| (h + hits, m + misses))
    }

    /// Shared textures currently held for `page`, displayed or awaiting GPU
    /// completion.
    fn outstanding_textures(&self, page: PageId) -> usize {
        let displayed = self.pages.get(&page).map_or(0, PageLayers::shared_count);
        displayed + self.retired.count_for(page)
    }

    /// Draws `scene` into `target` and submits the work.
    pub fn render(&mut self, target: &wgpu::TextureView, scene: &SceneView<'_>) -> RenderStats {
        let started = Instant::now();
        self.reclaim();

        let pages = &self.pages;
        let counts = build_draw_list(
            scene,
            |page| pages.get(&page).and_then(PageLayers::info),
            &mut self.instances,
            &mut self.draw_items,
        );
        let max_paint_to_submit = self.mark_shown(started);

        let grid = grid_metrics(&scene.camera, &scene.grid, scene.scale_factor);
        let uniforms = FrameUniforms::new(scene, &grid, !self.target_format.is_srgb());
        self.queue
            .write_buffer(&self.frame_uniforms, 0, bytemuck::bytes_of(&uniforms));
        self.ensure_instance_capacity(self.instances.len());
        if !self.instances.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.instances),
            );
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("specular-frame"),
            });
        self.encode_pass(&mut encoder, target);
        self.queue.submit([encoder.finish()]);
        self.submitted += 1;
        if !self.retired.is_empty() {
            let completed = Arc::clone(&self.completed);
            let serial = self.submitted;
            self.queue.on_submitted_work_done(move || {
                completed.fetch_max(serial, Ordering::Release);
            });
        }

        let ingested = std::mem::take(&mut self.ingested);
        let (outstanding_textures, max_outstanding_textures) = self
            .pages
            .keys()
            .map(|&page| self.outstanding_textures(page))
            .fold((0, 0), |(total, max), held| (total + held, max.max(held)));
        RenderStats {
            pages_without_texture: counts.pages_without_texture,
            cpu_textures: counts.cpu_textures,
            max_paint_to_submit,
            frames_received: ingested.frames_received,
            popup_frames: ingested.popup_frames,
            frames_dropped_for_pool_pressure: ingested.frames_dropped_for_pool_pressure,
            outstanding_textures: outstanding_textures as u32,
            max_outstanding_textures: max_outstanding_textures as u32,
        }
    }

    fn encode_pass(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("specular-canvas"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Clear rather than Load: the grid overwrites every pixel,
                    // and Clear lets tile-based GPUs skip reading the
                    // previous frame back.
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..wgpu::RenderPassDescriptor::default()
        });
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        pass.set_pipeline(&self.pipelines.grid);
        pass.draw(0..3, 0..1);
        if self.draw_items.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipelines.quad);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        for (instance, item) in (0_u32..).zip(&self.draw_items) {
            let Some(layer) = self
                .pages
                .get(&item.page)
                .and_then(|layers| layers.get(item.layer))
            else {
                continue;
            };
            pass.set_bind_group(1, &layer.bind_group, &[]);
            pass.draw(0..4, instance..instance + 1);
        }
    }

    /// Flags this frame's layers as shown; returns the longest
    /// paint-to-submit wait among those shown for the first time.
    fn mark_shown(&mut self, now: Instant) -> Option<Duration> {
        let mut longest: Option<Duration> = None;
        for item in &self.draw_items {
            let Some(layer) = self
                .pages
                .get_mut(&item.page)
                .and_then(|layers| layers.slot_mut(item.layer).as_mut())
            else {
                continue;
            };
            if !layer.shown {
                layer.shown = true;
                let waited = now.saturating_duration_since(layer.produced_at);
                longest = Some(longest.map_or(waited, |max| max.max(waited)));
            }
        }
        longest
    }

    fn ingest_frame(&mut self, event: FrameEvent) -> Result<(), CompositorError> {
        let FrameEvent {
            page,
            layer,
            frame,
            produced_at,
        } = event;
        let kind = match layer {
            FrameLayer::View => {
                self.ingested.frames_received += 1;
                LayerKind::View
            }
            FrameLayer::Popup { rect } => {
                self.ingested.popup_frames += 1;
                let popup = &mut self.pages.entry(page).or_default().popup;
                popup.rect = Some(rect);
                popup.visible = true;
                LayerKind::Popup
            }
        };
        match frame {
            PageFrame::Cpu(cpu) => self.ingest_cpu(page, kind, &cpu, produced_at),
            PageFrame::GpuShared(shared) => self.ingest_shared(page, kind, shared, produced_at),
        }
    }

    fn ingest_cpu(
        &mut self,
        page: PageId,
        kind: LayerKind,
        frame: &CpuFrame,
        produced_at: Instant,
    ) -> Result<(), CompositorError> {
        upload::validate_cpu_frame(frame, self.device.limits().max_texture_dimension_2d)
            .map_err(|source| CompositorError::Import { page, source })?;
        let srgb = self.target_format.is_srgb();
        let slot = self.pages.entry(page).or_default().slot_mut(kind);
        let reusable = slot
            .as_ref()
            .is_some_and(|layer| layer.shared.is_none() && layer.size == frame.size);
        if reusable && let Some(layer) = slot.as_mut() {
            upload::write_frame(&self.queue, &layer.texture, frame, false);
            layer.produced_at = produced_at;
            layer.shown = false;
            return Ok(());
        }
        // The CPU path assumes BGRA: that is what CEF `OnPaint` delivers.
        let format = upload::page_texture_format(specular_core::PixelFormat::Bgra8Unorm, srgb);
        let texture = upload::create_page_texture(&self.device, frame.size, format);
        upload::write_frame(&self.queue, &texture, frame, true);
        let bind_group = self.pipelines.texture_bind_group(&self.device, &texture);
        let previous = slot.replace(LayerTexture {
            texture,
            bind_group,
            size: frame.size,
            shared: None,
            produced_at,
            shown: false,
        });
        self.retire(page, previous);
        Ok(())
    }

    fn ingest_shared(
        &mut self,
        page: PageId,
        kind: LayerKind,
        shared: SharedTexture,
        produced_at: Instant,
    ) -> Result<(), CompositorError> {
        self.reclaim();
        let size = shared.size();
        upload::validate_frame_size(size, self.device.limits().max_texture_dimension_2d)
            .map_err(|source| CompositorError::Import { page, source })?;
        let format = upload::page_texture_format(shared.format(), self.target_format.is_srgb());
        let key = SurfaceKey {
            surface: shared.surface(),
            size,
            format,
        };
        let cache = self.imports.entry((page, kind)).or_default();
        // A resize rebuilds the producer's pool: every older surface is gone.
        cache.retain(|cached| cached.size == size);
        let (device, pipelines) = (&self.device, &self.pipelines);
        let imported = cache
            .get_or_import(key, || {
                let texture = import_shared(device, &shared, format)?;
                let bind_group = pipelines.texture_bind_group(device, &texture);
                Ok(ImportedTexture {
                    texture,
                    bind_group,
                })
            })
            .map_err(|source| CompositorError::Import { page, source })?;
        let layer = LayerTexture {
            texture: imported.texture.clone(),
            bind_group: imported.bind_group.clone(),
            size,
            shared: Some(shared),
            produced_at,
            shown: false,
        };
        let previous = self
            .pages
            .entry(page)
            .or_default()
            .slot_mut(kind)
            .replace(layer);
        self.retire(page, previous);
        Ok(())
    }

    /// Parks a replaced layer's shared surface until the GPU has finished
    /// every submission so far.
    fn retire(&mut self, page: PageId, layer: Option<LayerTexture>) {
        if let Some(shared) = layer.and_then(|layer| layer.shared) {
            self.retired.push(page, self.submitted, shared);
        }
    }

    /// Runs GPU completion callbacks and releases surfaces they cover.
    fn reclaim(&mut self) {
        if self.retired.is_empty() {
            return;
        }
        if let Err(error) = self.device.poll(wgpu::PollType::Poll) {
            tracing::warn!("device poll failed: {error}");
        }
        self.retired.reclaim(self.completed.load(Ordering::Acquire));
    }

    fn ensure_instance_capacity(&mut self, needed: usize) {
        if needed <= self.instance_capacity {
            return;
        }
        let capacity = needed.next_power_of_two();
        self.instance_buffer = create_instance_buffer(&self.device, capacity);
        self.instance_capacity = capacity;
    }
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quad-instances"),
        size: (capacity * size_of::<QuadInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
