//! Glyph text through glyphon: one atlas, one `TextRenderer` per text batch.
//!
//! Shaped buffers are cached by what shapes them and dropped a few seconds
//! after a run stops appearing. Canvas text is rasterised at the zoom
//! [`RasterHold`] picks; when that differs from the camera's zoom the batch
//! is laid out as if the camera were at the held zoom and the render pass
//! viewport stretches it to where it belongs.

use std::collections::HashMap;

use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, ColorMode, Family, FontSystem, Metrics, Resolution,
    Shaping, Style, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight,
    Wrap, cosmic_text::Align,
};
use specular_scene::{FontFamily, Rect, Size, Space, TextAlign, TextRun};

use super::place::ViewTransform;
use super::raster_hold::RasterHold;
use super::text_layout::{same_shaping, shaping_hash, text_rect};
use crate::pipeline::SCENE_SAMPLES;

/// Frames a shaped buffer outlives the last frame its run appeared in.
const KEEP_FRAMES: u64 = 240;

/// One run of a text batch, resolved for drawing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TextDraw<'a> {
    pub(crate) run: &'a TextRun,
    /// The item's clip in logical pixels, if it has one.
    pub(crate) clip: Option<Rect>,
    pub(crate) opacity: f32,
}

struct Shaped {
    run: TextRun,
    buffer: Buffer,
    size: Size,
    last_used: u64,
}

/// Everything glyph text needs. Built on the first frame that shows text,
/// because loading the system fonts takes a moment.
pub(crate) struct TextSystem {
    fonts: FontSystem,
    swash: SwashCache,
    atlas: TextAtlas,
    /// Sized so the pass viewport can stretch held canvas text.
    canvas_viewport: Viewport,
    screen_viewport: Viewport,
    renderers: Vec<TextRenderer>,
    shaped: HashMap<u64, Shaped>,
    frame: u64,
    hold: RasterHold,
    /// `camera zoom / raster zoom` for this frame's canvas text.
    stretch: f32,
    /// Canvas text's layout resolution this frame, in held physical pixels.
    canvas_resolution: [u32; 2],
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
    ) -> Self {
        let cache = Cache::new(device);
        // An sRGB target blends in linear light; any other takes the colours
        // as written, like the shapes around the text.
        let color_mode = if target_format.is_srgb() {
            ColorMode::Accurate
        } else {
            ColorMode::Web
        };
        Self {
            fonts: FontSystem::new(),
            swash: SwashCache::new(),
            atlas: TextAtlas::with_color_mode(device, queue, &cache, target_format, color_mode),
            canvas_viewport: Viewport::new(device, &cache),
            screen_viewport: Viewport::new(device, &cache),
            renderers: Vec::new(),
            shaped: HashMap::new(),
            frame: 0,
            hold: RasterHold::default(),
            stretch: 1.0,
            canvas_resolution: [1, 1],
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
        let Some((buffer, size)) = shape(&mut self.fonts, run) else {
            return Size::default();
        };
        self.shaped.insert(
            key,
            Shaped {
                run: run.clone(),
                buffer,
                size,
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
        let shaped = &self.shaped;
        let areas = draws.filter_map(|draw| {
            let shaped = shaped
                .get(&shaping_hash(draw.run))
                .filter(|shaped| same_shaping(&shaped.run, draw.run))?;
            let rect = view.rect(space, text_rect(draw.run, shaped.size));
            let color = draw.run.color;
            let alpha = (f32::from(color.a) * draw.opacity.clamp(0.0, 1.0)).round() as u8;
            Some(TextArea {
                buffer: &shaped.buffer,
                left: rect.x * to_layout,
                top: rect.y * to_layout,
                scale: view.scale(space) * to_layout,
                bounds: draw.clip.map_or(whole, |clip| TextBounds {
                    left: (clip.x * to_layout).round() as i32,
                    top: (clip.y * to_layout).round() as i32,
                    right: (clip.right() * to_layout).round() as i32,
                    bottom: (clip.bottom() * to_layout).round() as i32,
                }),
                default_color: GlyphColor::rgba(color.r, color.g, color.b, alpha),
                custom_glyphs: &[],
            })
        });
        let result = self.renderers[slot].prepare(
            device,
            queue,
            &mut self.fonts,
            &mut self.atlas,
            viewport,
            areas,
            &mut self.swash,
        );
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

    /// Ends a frame: frees atlas space and buffers no run has used lately.
    pub(crate) fn end_frame(&mut self) {
        self.atlas.trim();
        let frame = self.frame;
        self.shaped
            .retain(|_, shaped| frame - shaped.last_used <= KEEP_FRAMES);
    }
}

/// Shapes and wraps `run`; `None` when its metrics cannot be shaped.
fn shape(fonts: &mut FontSystem, run: &TextRun) -> Option<(Buffer, Size)> {
    let usable = |value: f32| value.is_finite() && value > 0.0;
    if !usable(run.size) || !usable(run.line_height) {
        return None;
    }
    let family = match &run.family {
        FontFamily::SansSerif => Family::SansSerif,
        FontFamily::Serif => Family::Serif,
        FontFamily::Monospace => Family::Monospace,
        FontFamily::Named(name) => Family::Name(name),
    };
    let attrs = Attrs::new()
        .family(family)
        .weight(Weight(run.weight))
        .style(if run.italic {
            Style::Italic
        } else {
            Style::Normal
        });
    let align = match run.align {
        // `None` lets right-to-left text start from the right.
        TextAlign::Left => None,
        TextAlign::Centre => Some(Align::Center),
        TextAlign::Right => Some(Align::Right),
    };
    let mut buffer = Buffer::new_empty(Metrics::new(run.size, run.line_height));
    buffer.set_wrap(if run.wrap_width.is_some() {
        Wrap::WordOrGlyph
    } else {
        Wrap::None
    });
    buffer.set_size(run.wrap_width, None);
    buffer.set_text(&run.text, &attrs, Shaping::Advanced, align);
    buffer.shape_until_scroll(fonts, false);
    let mut size = measure(&buffer);
    if run.wrap_width.is_none() && align.is_some() {
        // Lines align inside a width, so an unwrapped run is given the width
        // of its longest line and laid out again.
        buffer.set_size(Some(size.width), None);
        buffer.shape_until_scroll(fonts, false);
        size = Size::new(size.width, measure(&buffer).height);
    }
    Some((buffer, size))
}

fn measure(buffer: &Buffer) -> Size {
    buffer
        .layout_runs()
        .fold(Size::default(), |size, line| Size {
            width: size.width.max(line.line_w),
            height: size.height.max(line.line_top + line.line_height),
        })
}
