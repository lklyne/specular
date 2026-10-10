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

mod ingest;

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
    /// `App::set_text_measure`:
    /// the editor's caret, selection and wrapping then agree with the drawn
    /// glyphs.
    pub fn text_measure(&self) -> GlyphMeasure {
        self.scene_pass.text_measure()
    }

    /// How tall the rows of each Document drawn by the latest
    /// [`render_scene`](Self::render_scene) came out, in canvas units, for
    /// `Event::NoteHeights`. A
    /// Document that was off screen is not in it.
    pub fn column_heights(&self) -> &[(specular_scene::OwnerId, f32)] {
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
            | PageEvent::Crashed { .. }
            | PageEvent::Title { .. }
            | PageEvent::Favicon { .. }
            | PageEvent::Url { .. }
            | PageEvent::Loading { .. }
            | PageEvent::Scrolled { .. }
            | PageEvent::ScrollProgress { .. }
            | PageEvent::Pointed { .. }
            | PageEvent::Candidates { .. }
            | PageEvent::ElementAt { .. }
            | PageEvent::ElementsInRect { .. }
            | PageEvent::Inspected { .. }
            | PageEvent::ElementCaptured { .. }
            | PageEvent::ElementPlaces { .. }
            | PageEvent::DevtoolsTarget { .. } => Ok(()),
        }
    }

    /// Forgets a closed page; its shared surfaces are released once the GPU
    /// has finished with them.
    pub fn remove_page(&mut self, page: PageId) {
        if let Some(layers) = self.pages.remove(&page) {
            self.retire(page, layers.view);
            self.retire(page, layers.kept);
            self.retire(page, layers.popup.texture);
        }
        self.imports.remove(&(page, LayerKind::View));
        self.imports.remove(&(page, LayerKind::Popup));
    }

    /// Lets go of what pages that have stopped painting no longer need:
    /// every imported surface but the one each shows, and the surfaces the
    /// GPU has finished with. A view frame kept from another size goes too
    /// once it has been held a while and is not what is drawn. Rendering does the second on its own; call
    /// this on a timer for when nothing is being rendered. Returns how many
    /// imports were dropped.
    pub fn release_idle(&mut self, now: Instant) -> usize {
        let idle: Vec<_> = (self.pages.iter_mut())
            .filter_map(|(page, layers)| Some((*page, layers.take_idle_kept(now)?)))
            .collect();
        for (page, kept) in idle {
            self.retire(page, Some(kept));
        }
        self.reclaim();
        (self.imports.values_mut())
            .map(|cache| cache.settle(now))
            .sum()
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
            let Some(layers) = self.pages.get_mut(&item.page) else {
                continue;
            };
            layers.drew(item.layer);
            let Some(layer) = layers.slot_mut(item.layer).as_mut() else {
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
}
