//! Taking a page's frames in: CPU uploads, shared-surface imports, and
//! retiring what they replace.

use super::{
    Compositor, CompositorError, CpuFrame, FrameEvent, FrameLayer, ImportedTexture, Instant,
    LayerKind, LayerTexture, Ordering, PageFrame, PageId, SharedTexture, SurfaceKey, import_shared,
    surface_identity, upload,
};

impl Compositor {
    pub(super) fn ingest_frame(&mut self, event: FrameEvent) -> Result<(), CompositorError> {
        let FrameEvent {
            page,
            layer,
            viewport,
            frame,
            produced_at,
        } = event;
        let kind = match layer {
            FrameLayer::View => {
                self.ingested.frames_received += 1;
                let layers = self.pages.entry(page).or_default();
                let evicted = layers.make_room(viewport, produced_at);
                self.retire(page, evicted);
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
        cache.painted(produced_at);
        // A resize rebuilds the producer's pool: every older surface is gone.
        cache.retain(|cached| cached.size == size);
        let (device, pipelines) = (&self.device, &self.pipelines);
        let started = specular_core::ledger::now_us();
        let mut missed = false;
        let imported = cache
            .get_or_import(key, || {
                missed = true;
                let texture = import_shared(device, &shared, format)?;
                let bind_group = pipelines.texture_bind_group(device, &texture);
                Ok(ImportedTexture {
                    texture,
                    bind_group,
                })
            })
            .map_err(|source| CompositorError::Import { page, source })?;
        if missed && specular_core::ledger::enabled() {
            let took = specular_core::ledger::now_us().saturating_sub(started);
            specular_core::ledger::record(specular_core::ledger::Entry::Import(page.0, took));
        }
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
    pub(super) fn retire(&mut self, page: PageId, layer: Option<LayerTexture>) {
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
