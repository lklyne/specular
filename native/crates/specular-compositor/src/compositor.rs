//! The compositor's state: page frames and how they are ingested. Drawing
//! is in `scene_pass`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use specular_core::{
    CpuFrame, FrameEvent, FrameLayer, PageEvent, PageFrame, PageId, PixelSize, SharedTexture,
};

use crate::draw_list::{DrawItem, LayerKind};
use crate::error::CompositorError;
use crate::gpu_types::{FRAME_UNIFORMS_SIZE, QuadInstance, ShapeInstance};
use crate::import::{import_shared, surface_identity};
use crate::import_cache::ImportCache;
use crate::instance_buffer::InstanceBuffer;
use crate::layers::{LayerTexture, PageLayers};
use crate::pipeline::Pipelines;
use crate::retire::RetiredTextures;
use crate::scene::RenderStats;
use crate::scene_pass::{GlyphMeasure, ScenePass};
use crate::upload;

/// Identity of an imported shared surface: the same surface at the same
/// size and format imports to the same texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SurfaceKey {
    surface: u64,
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
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) target_format: wgpu::TextureFormat,
    pub(crate) pipelines: Pipelines,
    pub(crate) frame_uniforms: wgpu::Buffer,
    pub(crate) frame_bind_group: wgpu::BindGroup,
    pub(crate) instance_buffer: InstanceBuffer<QuadInstance>,
    pub(crate) shape_buffer: InstanceBuffer<ShapeInstance>,
    pub(crate) pages: HashMap<PageId, PageLayers>,
    retired: RetiredTextures<SharedTexture>,
    imports: HashMap<(PageId, LayerKind), ImportCache<SurfaceKey, ImportedTexture>>,
    /// Serial of the most recent submit; 0 before the first.
    submitted: u64,
    /// Highest serial the GPU has reported complete.
    completed: Arc<AtomicU64>,
    ingested: IngestCounts,
    pub(crate) instances: Vec<QuadInstance>,
    pub(crate) shape_instances: Vec<ShapeInstance>,
    pub(crate) draw_items: Vec<DrawItem>,
    pub(crate) scene_pass: ScenePass,
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
        let instance_buffer = InstanceBuffer::new(&device, "quad-instances");
        let shape_buffer = InstanceBuffer::new(&device, "shape-instances");
        let scene_pass = ScenePass::new(&device);
        Self {
            device,
            queue,
            target_format,
            pipelines,
            frame_uniforms,
            frame_bind_group,
            instance_buffer,
            shape_buffer,
            pages: HashMap::new(),
            retired: RetiredTextures::default(),
            imports: HashMap::new(),
            submitted: 0,
            completed: Arc::new(AtomicU64::new(0)),
            ingested: IngestCounts::default(),
            instances: Vec::new(),
            shape_instances: Vec::new(),
            draw_items: Vec::new(),
            scene_pass,
        }
    }

    /// Loads the system fonts and builds the glyph atlas now, so the first
    /// frame that shows text does not stall on it.
    pub fn warm_text(&mut self) {
        self.scene_pass
            .warm_text(&self.device, &self.queue, self.target_format);
    }

    /// A text measure on the fonts this compositor draws with, for
    /// [`App::set_text_measure`](specular_interact::App::set_text_measure):
    /// the editor's caret, selection and wrapping then agree with the drawn
    /// glyphs.
    pub fn text_measure(&self) -> GlyphMeasure {
        self.scene_pass.text_measure()
    }

    /// How tall the rows of each Document drawn by the latest
    /// [`render_scene`](Self::render_scene) came out, in canvas units, for
    /// [`Event::NoteHeights`](specular_interact::Event::NoteHeights). A
    /// Document that was off screen is not in it.
    pub fn column_heights(&self) -> &[(specular_doc::EntityId, f32)] {
        self.scene_pass.column_heights()
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

    /// Submits a frame's commands and asks to hear when the GPU has finished
    /// them, if any retired surface is waiting on that.
    pub(crate) fn submit(&mut self, encoder: wgpu::CommandEncoder) {
        self.queue.submit([encoder.finish()]);
        self.submitted += 1;
        if !self.retired.is_empty() {
            let completed = Arc::clone(&self.completed);
            let serial = self.submitted;
            self.queue.on_submitted_work_done(move || {
                completed.fetch_max(serial, Ordering::Release);
            });
        }
    }

    /// The counters every render reports, taking the ingest counts gathered
    /// since the previous one. The page counts are the caller's to fill in.
    pub(crate) fn take_frame_stats(
        &mut self,
        max_paint_to_submit: Option<Duration>,
    ) -> RenderStats {
        let ingested = std::mem::take(&mut self.ingested);
        let (outstanding_textures, max_outstanding_textures) = self
            .pages
            .keys()
            .map(|&page| self.outstanding_textures(page))
            .fold((0, 0), |(total, max), held| (total + held, max.max(held)));
        RenderStats {
            pages_without_texture: 0,
            cpu_textures: 0,
            shapes_drawn: self.shape_instances.len() as u32,
            max_paint_to_submit,
            frames_received: ingested.frames_received,
            popup_frames: ingested.popup_frames,
            frames_dropped_for_pool_pressure: ingested.frames_dropped_for_pool_pressure,
            outstanding_textures: outstanding_textures as u32,
            max_outstanding_textures: max_outstanding_textures as u32,
        }
    }

    /// Flags this frame's layers as shown; returns the longest
    /// paint-to-submit wait among those shown for the first time.
    pub(crate) fn mark_shown(&mut self, now: Instant) -> Option<Duration> {
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
            surface: surface_identity(&shared),
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
    pub(crate) fn reclaim(&mut self) {
        if self.retired.is_empty() {
            return;
        }
        if let Err(error) = self.device.poll(wgpu::PollType::Poll) {
            tracing::warn!("device poll failed: {error}");
        }
        self.retired.reclaim(self.completed.load(Ordering::Acquire));
    }
}
