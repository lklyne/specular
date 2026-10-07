//! Glyph text through glyphon: one atlas, one `TextRenderer` per text batch.
//!
//! Shaped buffers are cached by what shapes them and dropped a few seconds
//! after a run stops appearing. Canvas text is rasterised at the zoom
//! [`RasterHold`] picks; when that differs from the camera's zoom the batch
//! is laid out as if the camera were at the held zoom and the render pass
//! viewport stretches it to where it belongs.
//!
//! glyphon draws glyphs and nothing else. The straight lines that go with
//! text (underlines, strikes, and the rules of a column's rows) are sent
//! through its custom-glyph path as solid masks, so they share the batch, the
//! clip and the paint order of the text they belong to.

use std::collections::HashMap;

use glyphon::{
    Buffer, Cache, ColorMode, Metrics, Resolution, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
use specular_doc::EntityId;
use specular_scene::{Point, Size, Space, TextRun};

use super::place::ViewTransform;
use super::raster_hold::RasterHold;
use super::text_areas::{Areas, Shaped, solid};
pub(crate) use super::text_areas::{TextDraw, TextItem};
use super::text_layout::{same_shaping, shaping_hash};
use super::text_shape::shape;
use crate::fonts::Fonts;
use crate::pipeline::SCENE_SAMPLES;

/// Frames a shaped buffer outlives the last frame its run appeared in.
const KEEP_FRAMES: u64 = 240;

/// Everything glyph text needs. Built on the first frame that shows text,
/// because loading the system fonts takes a moment.
pub(crate) struct TextSystem {
    /// Shared with the editor's measure, so both shape with the same fonts.
    fonts: Fonts,
    swash: SwashCache,
    atlas: TextAtlas,
    /// Sized so the pass viewport can stretch held canvas text.
    canvas_viewport: Viewport,
    screen_viewport: Viewport,
    renderers: Vec<TextRenderer>,
    shaped: HashMap<u64, Shaped>,
    /// The buffer of an area that draws lines and no glyphs.
    blank: Buffer,
    frame: u64,
    hold: RasterHold,
    /// `camera zoom / raster zoom` for this frame's canvas text.
    stretch: f32,
    /// Canvas text's layout resolution this frame, in held physical pixels.
    canvas_resolution: [u32; 2],
    /// How tall each owned column drawn this frame came out, in its own
    /// units.
    column_heights: Vec<(EntityId, f32)>,
}

impl std::fmt::Debug for TextSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextSystem")
            .field("renderers", &self.renderers.len())
            .field("shaped", &self.shaped.len())
            .field("stretch", &self.stretch)
            .finish_non_exhaustive()
    }
}

impl TextSystem {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
        fonts: Fonts,
    ) -> Self {
        // Loads the system fonts now, if nothing has yet.
        fonts.with(|_| ());
        let cache = Cache::new(device);
        // An sRGB target blends in linear light; any other takes the colours
        // as written, like the shapes around the text.
        let color_mode = if target_format.is_srgb() {
            ColorMode::Accurate
        } else {
            ColorMode::Web
        };
        Self {
            fonts,
            swash: SwashCache::new(),
            atlas: TextAtlas::with_color_mode(device, queue, &cache, target_format, color_mode),
            canvas_viewport: Viewport::new(device, &cache),
            screen_viewport: Viewport::new(device, &cache),
            renderers: Vec::new(),
            shaped: HashMap::new(),
            blank: Buffer::new_empty(Metrics::new(1.0, 1.0)),
            frame: 0,
            hold: RasterHold::default(),
            stretch: 1.0,
            canvas_resolution: [1, 1],
            column_heights: Vec::new(),
        }
    }

    /// Starts a frame: picks the raster zoom and sizes the two viewports.
    pub(crate) fn begin_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &ViewTransform,
        zooming: bool,
    ) {
        self.frame += 1;
        self.column_heights.clear();
        let zoom = view.camera.zoom;
        let limit = device.limits().max_texture_dimension_2d as f32;
        let held = |stretch: f32| {
            view.target
                .map(|side| (side as f32 / stretch).ceil().max(1.0))
        };
        self.stretch = zoom / self.hold.raster_zoom(zoom, zooming);
        // The pass viewport that does the stretching cannot pass the device's
        // texture limit; if it would, rasterise at the real zoom after all.
        if held(self.stretch)
            .iter()
            .any(|side| side * self.stretch > limit)
        {
            self.hold.release(zoom);
            self.stretch = 1.0;
        }
        self.canvas_resolution = held(self.stretch).map(|side| side as u32);
        let [width, height] = self.canvas_resolution;
        self.canvas_viewport
            .update(queue, Resolution { width, height });
        let [width, height] = view.target;
        self.screen_viewport
            .update(queue, Resolution { width, height });
    }

    /// The size of `run`'s shaped lines in its own units, shaping it if no
    /// cached buffer matches. A run that cannot be shaped measures zero.
    pub(crate) fn measure(&mut self, run: &TextRun) -> Size {
        let frame = self.frame;
        let key = shaping_hash(run);
        if let Some(shaped) = self.shaped.get_mut(&key)
            && same_shaping(&shaped.run, run)
        {
            shaped.last_used = frame;
            return shaped.size;
        }
        let Some(shaped) = self.fonts.with(|fonts| shape(fonts, run)) else {
            return Size::default();
        };
        let size = shaped.size;
        self.shaped.insert(
            key,
            Shaped {
                run: run.clone(),
                buffer: shaped.buffer,
                size,
                lines: shaped.lines,
                last_used: frame,
            },
        );
        size
    }

    /// Lays out the glyph quads of one batch into renderer `slot`. Every run
    /// must have been [`measure`](Self::measure)d this frame.
    pub(crate) fn prepare<'a>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &ViewTransform,
        slot: usize,
        space: Space,
        draws: impl Iterator<Item = TextDraw<'a>>,
    ) {
        while self.renderers.len() <= slot {
            let multisample = wgpu::MultisampleState {
                count: SCENE_SAMPLES,
                ..wgpu::MultisampleState::default()
            };
            self.renderers.push(TextRenderer::new(
                &mut self.atlas,
                device,
                multisample,
                None,
            ));
        }
        let (stretch, resolution, viewport) = match space {
            Space::Canvas => (self.stretch, self.canvas_resolution, &self.canvas_viewport),
            Space::Screen => (1.0, view.target, &self.screen_viewport),
        };
        // Logical pixels on screen to the physical pixels glyphon lays out in.
        let to_layout = view.scale_factor / stretch;
        let whole = TextBounds {
            left: 0,
            top: 0,
            right: resolution[0] as i32,
            bottom: resolution[1] as i32,
        };
        let scale = view.scale(space) * to_layout;
        let hairline = 1.0 / scale.max(f32::EPSILON);
        let mut areas = Areas::new(&self.shaped, hairline, whole, to_layout);
        for draw in draws {
            match draw.text {
                TextItem::Run(run) => areas.run(run, Point::default(), &draw),
                TextItem::Column(column) => {
                    let visible = draw.clip.map(|clip| view.rect_to(space, clip));
                    areas.column(column, visible, &draw);
                }
            }
        }
        let (areas, lines, heights) = areas.finish();
        self.column_heights.extend(heights);
        let blank = &self.blank;
        let areas = areas.iter().map(|area| {
            let origin = view.point(space, area.origin);
            TextArea {
                buffer: area.buffer.unwrap_or(blank),
                left: origin.x * to_layout,
                top: origin.y * to_layout,
                scale,
                bounds: area.bounds,
                default_color: area.color,
                custom_glyphs: &lines[area.lines.clone()],
            }
        });
        let (renderer, atlas, swash) =
            (&mut self.renderers[slot], &mut self.atlas, &mut self.swash);
        let result = self.fonts.with(|fonts| {
            renderer.prepare_with_custom(device, queue, fonts, atlas, viewport, areas, swash, solid)
        });
        if let Err(error) = result {
            tracing::warn!("text batch not prepared: {error}");
        }
    }

    /// Draws the batch prepared into `slot`. The caller restores its own
    /// pipeline, bind groups and viewport afterwards.
    pub(crate) fn render(&self, pass: &mut wgpu::RenderPass<'_>, slot: usize, space: Space) {
        let (viewport, size) = match space {
            Space::Canvas => (
                &self.canvas_viewport,
                self.canvas_resolution
                    .map(|side| side as f32 * self.stretch),
            ),
            Space::Screen => (
                &self.screen_viewport,
                [
                    self.screen_viewport.resolution().width as f32,
                    self.screen_viewport.resolution().height as f32,
                ],
            ),
        };
        let Some(renderer) = self.renderers.get(slot) else {
            return;
        };
        pass.set_viewport(0.0, 0.0, size[0], size[1], 0.0, 1.0);
        if let Err(error) = renderer.render(&self.atlas, viewport, pass) {
            tracing::warn!("text batch not drawn: {error}");
        }
    }

    /// The height of the rows of each column drawn this frame that names an
    /// owner.
    pub(crate) fn column_heights(&self) -> &[(EntityId, f32)] {
        &self.column_heights
    }

    /// Ends a frame: frees atlas space and buffers no run has used lately.
    pub(crate) fn end_frame(&mut self) {
        self.atlas.trim();
        let frame = self.frame;
        self.shaped
            .retain(|_, shaped| frame - shaped.last_used <= KEEP_FRAMES);
    }
}
