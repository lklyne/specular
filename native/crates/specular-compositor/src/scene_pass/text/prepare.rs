//! Laying a space's text batches out, or reusing what is held.

use super::{
    Areas, Laid, Placement, Resolution, SCENE_SAMPLES, Slot, Space, TextArea, TextBatch,
    TextCounts, TextFrame, TextItem, TextRenderer, TextSystem, Viewport, member_key, solid,
};

impl TextSystem {
    pub(super) fn prepare_layer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &TextFrame,
        batches: &[&TextBatch<'_>],
        moving: bool,
        counts: &mut TextCounts,
    ) {
        let cache = &self.cache;
        let layer = match frame.space {
            Space::Canvas => &mut self.canvas,
            Space::Screen => &mut self.screen,
        };
        while layer.slots.len() < batches.len() {
            let multisample = wgpu::MultisampleState {
                count: SCENE_SAMPLES,
                ..wgpu::MultisampleState::default()
            };
            layer.slots.push(Slot {
                renderer: TextRenderer::new(&mut layer.atlas, device, multisample, None),
                viewport: Viewport::new(device, cache),
                laid: None,
                placement: None,
                glyphs: 0,
                heights: Vec::new(),
            });
        }
        // A slot with no batch this frame holds nothing worth keeping.
        for slot in &mut layer.slots[batches.len()..] {
            slot.laid = None;
            slot.placement = None;
        }
        let plans: Vec<(Vec<u64>, Option<Placement>)> = (batches.iter())
            .zip(&layer.slots)
            .map(|(batch, slot)| {
                let members: Vec<u64> = batch.draws.iter().map(member_key).collect();
                let placement = (slot.laid.as_ref())
                    .and_then(|laid| laid.placement(frame, &members))
                    .filter(|placement| placement.exact || moving);
                (members, placement)
            })
            .collect();
        // Glyphs leave an atlas only between a trim and the next layout that
        // needs room. So one stale batch lays the whole space out again,
        // after a trim: nothing kept then points at an unmarked glyph.
        let all = plans.iter().any(|(_, placement)| placement.is_none());
        if all {
            layer.atlas.trim();
        }
        let unstretched = (frame.stretch - 1.0).abs() < f32::EPSILON;
        for (index, (batch, (members, placement))) in batches.iter().zip(plans).enumerate() {
            if let Some(placement) = placement.filter(|_| !all) {
                self.layer_mut(frame.space).slots[index].placement = Some(placement);
                counts.reused += 1;
                counts.settling |= !placement.exact && unstretched;
            } else {
                self.lay_out(device, queue, index, frame, members, batch);
                counts.laid_out += 1;
            }
            let slot = &self.layer_mut(frame.space).slots[index];
            counts.glyphs += slot.glyphs;
            let heights = slot.heights.clone();
            self.column_heights.extend(heights);
        }
    }

    /// Lays out the glyph quads of `batch` into slot `index` of its space.
    fn lay_out(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        index: usize,
        frame: &TextFrame,
        members: Vec<u64>,
        batch: &TextBatch<'_>,
    ) {
        let laid = Laid::new(frame, members);
        let hairline = 1.0 / laid.scale().max(f32::EPSILON);
        let mut areas = Areas::new(&self.shaped, hairline, &laid);
        for draw in &batch.draws {
            match draw.text {
                TextItem::Run(run) => areas.run(run, specular_scene::Point::default(), draw),
                TextItem::Column(column) => areas.column(column, draw),
            }
        }
        let glyphs = areas.glyphs();
        let (areas, lines, heights) = areas.finish();
        let blank = &self.blank;
        let areas = areas.iter().map(|area| {
            let origin = laid.point(area.origin.x, area.origin.y);
            TextArea {
                buffer: area.buffer.unwrap_or(blank),
                left: origin.x,
                top: origin.y,
                scale: laid.scale(),
                bounds: area.bounds,
                default_color: area.color,
                custom_glyphs: &lines[area.lines.clone()],
            }
        });
        let layer = match frame.space {
            Space::Canvas => &mut self.canvas,
            Space::Screen => &mut self.screen,
        };
        let slot = &mut layer.slots[index];
        let [width, height] = laid.size();
        slot.viewport.update(queue, Resolution { width, height });
        let (renderer, viewport) = (&mut slot.renderer, &slot.viewport);
        let (atlas, swash) = (&mut layer.atlas, &mut self.swash);
        let result = self.fonts.with(|fonts| {
            renderer.prepare_with_custom(device, queue, fonts, atlas, viewport, areas, swash, solid)
        });
        if let Err(error) = result {
            tracing::warn!("text batch not prepared: {error}");
        }
        slot.placement = laid.placement(frame, &[]);
        slot.laid = Some(laid);
        slot.glyphs = glyphs;
        slot.heights = heights;
    }
}
