//! The canvas surface: the compositor drawing into the `CAMetalLayer` of the
//! view under GPUI's.
//!
//! The view fills the window and never moves, so nothing can disagree with
//! GPUI during a resize. The canvas shows only through the slot GPUI leaves
//! unpainted, and that slot is the app's viewport: the frame is drawn with
//! the camera and the screen-space items shifted by the slot's corner.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use anyhow::Context as _;
use glam::Vec2;
use specular_app::offscreen::Target;
use specular_app::{PageOf, ShellWindow};
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext, SceneStats};
use specular_core::Camera;
use specular_interact::Cursor;
use specular_scene::{Draw, Item, Scene, Space};

use crate::native::NativeCanvas;
use crate::pacing::Pacing;

/// What effects ask of the window that only GPUI can do. The surface notes
/// it here and the canvas slot applies it on GPUI's next frame.
#[derive(Debug)]
pub(crate) struct WindowAsks {
    /// The pointer's shape over the canvas.
    pub(crate) cursor: Cell<Cursor>,
    /// Whether the input method may compose into the canvas.
    pub(crate) ime_allowed: Cell<bool>,
    /// The caret, as a corner and a size in logical pixels of the viewport.
    pub(crate) ime_area: Cell<(Vec2, Vec2)>,
    /// The window's title, which the toolbar shows: the title bar is hidden
    /// under it.
    pub(crate) title: RefCell<String>,
    /// Set when one of the above changed, until GPUI has been told.
    pub(crate) changed: Cell<bool>,
}

impl Default for WindowAsks {
    fn default() -> Self {
        Self {
            cursor: Cell::new(Cursor::Default),
            ime_allowed: Cell::new(false),
            ime_area: Cell::new((Vec2::ZERO, Vec2::ZERO)),
            title: RefCell::new(String::new()),
            changed: Cell::new(false),
        }
    }
}

/// Everything needed to put a canvas frame on screen.
pub(crate) struct CanvasSurface {
    native: NativeCanvas,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    context: GpuContext,
    compositor: Compositor,
    /// The view's size in points.
    size: Vec2,
    scale: f32,
    /// The slot's corner in the view, in points.
    slot_origin: Vec2,
    /// The slot's size in points: the app's viewport.
    slot_size: Vec2,
    asks: Rc<WindowAsks>,
    /// Whether the last render presented, so a change is logged once.
    presenting: bool,
    /// Frame times, when the run asked for them.
    pub(crate) pacing: Option<Pacing>,
}

impl std::fmt::Debug for CanvasSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanvasSurface")
            .field("size", &self.size)
            .field("scale", &self.scale)
            .field("slot_origin", &self.slot_origin)
            .field("slot_size", &self.slot_size)
            .finish_non_exhaustive()
    }
}

fn pixels(points: f32, scale: f32) -> u32 {
    ((points * scale).round() as u32).max(1)
}

impl CanvasSurface {
    /// A surface on `native`'s layer, with the whole view as the slot until
    /// GPUI has laid the window out.
    pub(crate) fn new(native: NativeCanvas, asks: Rc<WindowAsks>) -> anyhow::Result<Self> {
        let scale = native.scale();
        native.set_contents_scale(scale);
        let (width, height) = native.content_size();
        let size = Vec2::new(width as f32, height as f32);
        let scale = scale as f32;
        let instance = wgpu::Instance::default();
        // SAFETY: the layer is a live `CAMetalLayer` that the canvas view
        // retains for as long as the window, and so this surface, exists.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                native.layer().cast(),
            ))
        }
        .context("creating a surface on the canvas view's layer")?;
        let context = pollster::block_on(GpuContext::new(instance, Some(&surface)))?;
        let mut config = surface
            .get_default_config(
                &context.adapter,
                pixels(size.x, scale),
                pixels(size.y, scale),
            )
            .context("surface unsupported by adapter")?;
        // Not an sRGB format: the compositor then blends encoded colours, as
        // a browser does.
        let plain = config.format.remove_srgb_suffix();
        if surface
            .get_capabilities(&context.adapter)
            .formats
            .contains(&plain)
        {
            config.format = plain;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&context.device, &config);
        let mut compositor =
            Compositor::new(context.device.clone(), context.queue.clone(), config.format);
        // Loading the system fonts takes a moment. Better here than on the
        // first frame that shows text.
        compositor.warm_text();
        tracing::info!(
            adapter = %context.adapter.get_info().name,
            format = ?config.format,
            present_mode = ?config.present_mode,
            window_number = native.window_number(),
            "GPU ready"
        );
        Ok(Self {
            native,
            surface,
            config,
            context,
            compositor,
            size,
            scale,
            slot_origin: Vec2::ZERO,
            slot_size: size,
            asks,
            presenting: true,
            pacing: None,
        })
    }

    /// The native view and window.
    pub(crate) fn native(&self) -> &NativeCanvas {
        &self.native
    }

    /// Where GPUI laid the canvas slot out, in points from the window's
    /// content corner. Returns whether the viewport changed size.
    pub(crate) fn set_slot(&mut self, origin: Vec2, size: Vec2) -> bool {
        let resized = size != self.slot_size;
        self.slot_origin = origin;
        self.slot_size = size;
        resized
    }

    /// The slot's corner in the window, in points.
    pub(crate) fn slot_origin(&self) -> Vec2 {
        self.slot_origin
    }

    /// Brings the surface in step with the view's size and the display's
    /// scale. Returns the new scale when the display's changed.
    pub(crate) fn sync(&mut self) -> Option<f32> {
        let (width, height) = self.native.content_size();
        let size = Vec2::new(width as f32, height as f32);
        let native_scale = self.native.scale();
        let scale = native_scale as f32;
        if size == self.size && (scale - self.scale).abs() < f32::EPSILON {
            return None;
        }
        let rescaled = (scale - self.scale).abs() >= f32::EPSILON;
        self.size = size;
        self.scale = scale;
        self.native.set_contents_scale(native_scale);
        self.config.width = pixels(size.x, scale);
        self.config.height = pixels(size.y, scale);
        self.surface.configure(&self.context.device, &self.config);
        rescaled.then_some(scale)
    }

    /// Logs when frames stop or resume reaching the screen.
    fn note_presenting(&mut self, presenting: bool, reason: &str) {
        if presenting == self.presenting {
            return;
        }
        self.presenting = presenting;
        if presenting {
            tracing::info!("presenting frames again");
        } else {
            tracing::warn!(reason, "not presenting frames");
        }
    }

    /// Stops the display link and removes the view.
    pub(crate) fn close(&mut self) {
        self.native.close();
    }
}

/// `item` moved by `by` on screen. A column keeps the entity it reports its
/// height for: this is the same item, not a copy of it somewhere else.
fn shifted(item: Item, by: Vec2) -> Item {
    let owner = match &item.draw {
        Draw::Column(column) => column.owner.clone(),
        _ => None,
    };
    let mut moved = item.translated(by.x, by.y);
    if let Draw::Column(column) = &mut moved.draw {
        column.owner = owner;
    }
    moved
}

/// Moves every screen-space item of `scene` by `by`.
fn shift_screen_items(scene: &mut Scene, by: Vec2) {
    if by == Vec2::ZERO {
        return;
    }
    let items = std::mem::take(&mut scene.items);
    scene.items = (items.into_iter())
        .map(|item| match item.space {
            Space::Screen => shifted(item, by),
            Space::Canvas => item,
        })
        .collect();
}

impl ShellWindow for CanvasSurface {
    fn compositor(&self) -> &Compositor {
        &self.compositor
    }

    fn compositor_mut(&mut self) -> &mut Compositor {
        &mut self.compositor
    }

    fn scale_factor(&self) -> f32 {
        self.scale
    }

    fn logical_viewport(&self) -> Vec2 {
        self.slot_size
    }

    fn render(
        &mut self,
        camera: Camera,
        zooming: bool,
        scene: &mut Scene,
        page_of: PageOf<'_>,
    ) -> Option<SceneStats> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.context.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.context.device, &self.config);
                self.note_presenting(false, "surface outdated or lost");
                return None;
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                self.note_presenting(false, "window occluded");
                return None;
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.note_presenting(false, "surface timed out");
                return None;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                self.note_presenting(false, "surface validation error");
                return None;
            }
        };
        self.note_presenting(true, "");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // The app's screen starts at the slot's corner; the surface's starts
        // at the window's.
        shift_screen_items(scene, self.slot_origin);
        let frame_view = FrameView {
            camera: Camera {
                pan: camera.pan + self.slot_origin,
                zoom: camera.zoom,
            },
            viewport: self.size,
            scale_factor: self.scale,
            grid: DotGrid::default(),
            zooming,
        };
        let stats = self
            .compositor
            .render_scene(&view, &frame_view, scene, page_of);
        self.context.queue.present(frame);
        if let Some(pacing) = self.pacing.as_mut() {
            pacing.mark();
        }
        Some(stats)
    }

    fn capture(
        &mut self,
        camera: Camera,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        self.capture_area(camera, self.slot_size, scene, page_of)
    }

    fn capture_area(
        &mut self,
        camera: Camera,
        viewport: Vec2,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let target = Target::new(
            &self.context,
            pixels(viewport.x, self.scale),
            pixels(viewport.y, self.scale),
            self.config.format,
        );
        let frame_view = FrameView {
            camera,
            viewport,
            scale_factor: self.scale,
            grid: DotGrid::default(),
            zooming: false,
        };
        self.compositor
            .render_scene(&target.view(), &frame_view, scene, page_of);
        let (width, height) = target.size();
        Ok((target.png(&self.context)?, width, height))
    }

    fn set_ime_allowed(&self, allowed: bool) {
        if self.asks.ime_allowed.replace(allowed) != allowed {
            self.asks.changed.set(true);
        }
    }

    fn set_ime_cursor_area(&self, origin: Vec2, size: Vec2) {
        self.asks.ime_area.set((origin, size));
    }

    fn set_cursor(&self, cursor: Cursor) {
        if self.asks.cursor.replace(cursor) != cursor {
            self.asks.changed.set(true);
        }
    }

    fn set_title(&self, title: &str, unsaved: bool) {
        self.native.set_title(title, unsaved);
        title.clone_into(&mut self.asks.title.borrow_mut());
        self.asks.changed.set(true);
    }
}

#[cfg(test)]
mod tests {
    use specular_scene::{Color, Rect, RectDraw};

    use super::*;

    #[test]
    fn only_screen_space_items_move_with_the_slot() {
        let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
        let mut scene = Scene::new();
        scene.push(Item::canvas(RectDraw::filled(rect, Color::WHITE)));
        scene.push(Item::screen(RectDraw::filled(rect, Color::WHITE)).clipped(rect));
        shift_screen_items(&mut scene, Vec2::new(256.0, 0.0));
        let there = Rect::new(266.0, 20.0, 30.0, 40.0);
        assert_eq!(scene.items[0].draw.bounds(), Some(rect));
        assert_eq!(
            (scene.items[1].draw.bounds(), scene.items[1].clip),
            (Some(there), Some(there))
        );
    }
}
