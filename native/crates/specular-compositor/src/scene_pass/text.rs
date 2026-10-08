//! Glyph text through glyphon: an atlas for each space, and a `TextRenderer`
//! per text batch.
//!
//! Shaped buffers are cached by what shapes them and dropped a few seconds
//! after a run stops appearing. A batch's glyph quads are laid out once and
//! kept: later frames draw them through a pass viewport that follows the
//! camera, until the batch changes or the camera leaves what was laid out
//! (see [`text_hold`](super::text_hold)). Canvas text is rasterised at the
//! zoom [`RasterHold`] picks, and the same viewport stretches it when that
//! differs from the camera's zoom.
//!
//! Canvas and screen text keep separate atlases because they go stale at
//! different times: screen text is placed by the camera and laid out on
//! every frame it moves, canvas text almost never. An atlas can only let
//! glyphs go when nothing kept still points at them, which is when every
//! batch drawn from it is laid out again.
//!
//! glyphon draws glyphs and nothing else. The straight lines that go with
//! text (underlines, strikes, and the rules of a column's rows) are sent
//! through its custom-glyph path as solid masks, so they share the batch, the
//! clip and the paint order of the text they belong to.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use glam::Vec2;
use glyphon::{
    Buffer, Cache, ColorMode, Metrics, Resolution, SwashCache, TextArea, TextAtlas, TextRenderer,
    Viewport,
};
use specular_core::Camera;
use specular_doc::EntityId;
use specular_scene::{Size, Space, TextRun};

use super::place::ViewTransform;
use super::raster_hold::RasterHold;
use super::text_areas::{Areas, Shaped, solid};
pub(crate) use super::text_areas::{TextDraw, TextItem};
use super::text_hold::{Laid, Placement, TextFrame};
use super::text_key::member_key;
use super::text_layout::{same_shaping, shaping_hash};
use super::text_shape::shape;
use crate::fonts::Fonts;
use crate::pipeline::SCENE_SAMPLES;

mod prepare;

/// Frames a shaped buffer outlives the last frame its run appeared in.
const KEEP_FRAMES: u64 = 240;
/// The items of one text batch, in paint order.
#[derive(Debug)]
pub(crate) struct TextBatch<'a> {
    pub(crate) space: Space,
    pub(crate) draws: Vec<TextDraw<'a>>,
}

/// What the text system did with a frame's batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextCounts {
    /// Batches laid out this frame.
    pub laid_out: u32,
    /// Batches drawn from an earlier frame's layout.
    pub reused: u32,
    /// Glyphs in the batches drawn, laid out now or earlier.
    pub glyphs: u32,
    /// Whether a batch was drawn up to half a pixel off, as a pan by part
    /// of a pixel leaves it. A frame with the camera at rest puts it right.
    pub settling: bool,
}

/// One batch's renderer and the layout it holds.
struct Slot {
    renderer: TextRenderer,
    viewport: Viewport,
    laid: Option<Laid>,
    /// Where this frame draws the layout.
    placement: Option<Placement>,
    glyphs: u32,
    /// How tall each owned column of the layout came out.
    heights: Vec<(EntityId, f32)>,
}

/// The text of one space: its glyph atlas and a slot for each batch.
struct Layer {
    atlas: TextAtlas,
    slots: Vec<Slot>,
}

/// Everything glyph text needs. Built on the first frame that shows text,
/// because loading the system fonts takes a moment.
pub(crate) struct TextSystem {
    /// Shared with the editor's measure, so both shape with the same fonts.
    fonts: Fonts,
    swash: SwashCache,
    cache: Cache,
    canvas: Layer,
    screen: Layer,
    shaped: HashMap<u64, Shaped>,
    /// The buffer of an area that draws lines and no glyphs.
    blank: Buffer,
    frame: u64,
    hold: RasterHold,
    /// The zoom this frame's canvas text is rasterised at, and
    /// `camera zoom / raster zoom`.
    raster_zoom: f32,
    stretch: f32,
    /// The camera the last frame was drawn under.
    last_camera: Option<Camera>,
    /// How tall each owned column drawn this frame came out, in its own
    /// units.
    column_heights: Vec<(EntityId, f32)>,
    /// Time spent shaping this frame.
    shaping_time: Duration,
}

impl std::fmt::Debug for TextSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextSystem")
            .field("canvas_slots", &self.canvas.slots.len())
            .field("screen_slots", &self.screen.slots.len())
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
        let layer = || Layer {
            atlas: TextAtlas::with_color_mode(device, queue, &cache, target_format, color_mode),
            slots: Vec::new(),
        };
        Self {
            fonts,
            swash: SwashCache::new(),
            canvas: layer(),
            screen: layer(),
            cache,
            shaped: HashMap::new(),
            blank: Buffer::new_empty(Metrics::new(1.0, 1.0)),
            frame: 0,
            hold: RasterHold::default(),
            raster_zoom: 1.0,
            stretch: 1.0,
            last_camera: None,
            column_heights: Vec::new(),
            shaping_time: Duration::ZERO,
        }
    }

    /// Starts a frame: picks the zoom canvas text is rasterised at.
    pub(crate) fn begin_frame(
        &mut self,
        device: &wgpu::Device,
        view: &ViewTransform,
        zooming: bool,
    ) {
        self.frame += 1;
        self.column_heights.clear();
        self.shaping_time = Duration::ZERO;
        let zoom = view.camera.zoom;
        self.raster_zoom = self.hold.raster_zoom(zoom, zooming);
        self.stretch = zoom / self.raster_zoom;
        // The pass viewport that does the stretching cannot pass the device's
        // texture limit; if it would, rasterise at the real zoom after all.
        let limit = device.limits().max_texture_dimension_2d as f32;
        let stretched = |side: u32| (side as f32 / self.stretch).ceil().max(1.0) * self.stretch;
        if view.target.iter().any(|&side| stretched(side) > limit) {
            self.hold.release(zoom);
            self.raster_zoom = zoom;
            self.stretch = 1.0;
        }
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
        let started = Instant::now();
        let shaped = self.fonts.with(|fonts| shape(fonts, run));
        self.shaping_time += started.elapsed();
        let Some(shaped) = shaped else {
            return Size::default();
        };
        let size = shaped.size;
        self.shaped.insert(
            key,
            Shaped {
                run: run.clone(),
                glyphs: glyph_count(&shaped.buffer),
                buffer: shaped.buffer,
                size,
                lines: shaped.lines,
                last_used: frame,
            },
        );
        size
    }

    /// How this frame lays out and places the text of `space`.
    fn text_frame(&self, device: &wgpu::Device, view: &ViewTransform, space: Space) -> TextFrame {
        // The raster zoom itself, not the camera's zoom over the stretch:
        // the scale must be the same number on every frame of a held zoom.
        let (zoom, stretch, origin) = match space {
            Space::Canvas => (
                self.raster_zoom,
                self.stretch,
                view.camera.pan * view.scale_factor,
            ),
            Space::Screen => (1.0, 1.0, Vec2::ZERO),
        };
        TextFrame {
            space,
            scale: zoom * view.scale_factor,
            stretch,
            origin,
            target: view.target,
            limit: device.limits().max_texture_dimension_2d as f32,
        }
    }

    /// Gets every batch ready to draw, the `n`th batch of a space in that
    /// space's slot `n`: placed from the layout the slot holds where that
    /// still fits, laid out again where it does not. Every run must have
    /// been [`measure`](Self::measure)d this frame.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &ViewTransform,
        batches: &[TextBatch<'_>],
    ) -> TextCounts {
        // At rest, a layout drawn off the pixel is put right.
        let moving = self.last_camera != Some(view.camera);
        self.last_camera = Some(view.camera);
        let mut counts = TextCounts::default();
        for space in [Space::Canvas, Space::Screen] {
            let frame = self.text_frame(device, view, space);
            let batches: Vec<&TextBatch<'_>> = (batches.iter())
                .filter(|batch| batch.space == space)
                .collect();
            self.prepare_layer(device, queue, &frame, &batches, moving, &mut counts);
        }
        counts
    }

    fn layer_mut(&mut self, space: Space) -> &mut Layer {
        match space {
            Space::Canvas => &mut self.canvas,
            Space::Screen => &mut self.screen,
        }
    }

    /// Draws the batch in `slot` of `space`. The caller restores its own
    /// pipeline, bind groups and viewport afterwards.
    pub(crate) fn render(&self, pass: &mut wgpu::RenderPass<'_>, slot: usize, space: Space) {
        let layer = match space {
            Space::Canvas => &self.canvas,
            Space::Screen => &self.screen,
        };
        let Some(slot) = layer.slots.get(slot) else {
            return;
        };
        let Some(placed) = slot.placement else {
            return;
        };
        pass.set_viewport(placed.x, placed.y, placed.width, placed.height, 0.0, 1.0);
        if let Err(error) = slot.renderer.render(&layer.atlas, &slot.viewport, pass) {
            tracing::warn!("text batch not drawn: {error}");
        }
    }

    /// The height of the rows of each column drawn this frame that names an
    /// owner.
    pub(crate) fn column_heights(&self) -> &[(EntityId, f32)] {
        &self.column_heights
    }

    /// Time spent shaping since the frame began.
    pub(crate) fn shaping_time(&self) -> Duration {
        self.shaping_time
    }

    /// Ends a frame: frees buffers no run has used lately.
    pub(crate) fn end_frame(&mut self) {
        let frame = self.frame;
        self.shaped
            .retain(|_, shaped| frame - shaped.last_used <= KEEP_FRAMES);
    }
}

fn glyph_count(buffer: &Buffer) -> u32 {
    buffer
        .layout_runs()
        .map(|run| run.glyphs.len())
        .sum::<usize>() as u32
}
