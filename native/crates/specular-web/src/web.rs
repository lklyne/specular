//! The browser half: [`WebViewer`], what the web page calls. It owns the
//! canvas element's GPU surface and the compositor, turns DOM input into
//! the viewer's, and draws a frame when one is asked for and something
//! changed.
//!
//! WebGPU is used where the browser has it and WebGL2 everywhere else.

use std::sync::Arc;

use glam::Vec2;
use js_sys::Uint8Array;
use specular_compositor::{Compositor, DotGrid, FrameView};
use specular_core::{
    FrameEvent, FrameLayer, Modifiers, PageEvent, PageFrame, PixelSize, PointerButton,
    PointerEventKind,
};
use specular_scene::{ImageId, ViewCache};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use web_time::Instant;

use crate::keys::key_input;
use specular_doc::AnnotationId;
use specular_interact::ImageKey;

use crate::viewer::{Access, Mode, Upload, Viewer};

/// The GPU the canvas element draws with.
#[derive(Debug)]
struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    compositor: Compositor,
}

/// A published canvas in a `<canvas>` element. See the module docs.
#[wasm_bindgen]
#[derive(Debug)]
pub struct WebViewer {
    viewer: Viewer,
    gpu: Gpu,
    cache: ViewCache,
    viewport: Vec2,
    scale: f32,
    drawn_zoom: f32,
}

/// Modifier keys as the page packs them: shift 1, control 2, alt 4, meta 8.
fn modifiers(bits: u8) -> Modifiers {
    Modifiers {
        shift: bits & 1 != 0,
        control: bits & 2 != 0,
        alt: bits & 4 != 0,
        meta: bits & 8 != 0,
    }
}

/// A DOM `MouseEvent.button`.
fn button(index: u8) -> PointerButton {
    match index {
        1 => PointerButton::Middle,
        2 => PointerButton::Right,
        _ => PointerButton::Left,
    }
}

fn error(message: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&message.to_string())
}

#[wasm_bindgen]
impl WebViewer {
    /// Opens `canvas_text` in `canvas`. `can_comment` is what the link
    /// allows; `seed` starts the ids of comments made here.
    pub async fn create(
        canvas: HtmlCanvasElement,
        canvas_text: String,
        can_comment: bool,
        seed: f64,
    ) -> Result<WebViewer, JsValue> {
        console_error_panic_hook::set_once();
        let (width, height) = (canvas.width().max(1), canvas.height().max(1));
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
        .await;
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..wgpu::RequestAdapterOptions::default()
            })
            .await
            .map_err(error)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("specular-web"),
                required_limits: adapter.limits(),
                ..wgpu::DeviceDescriptor::default()
            })
            .await
            .map_err(error)?;
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| error("the canvas cannot be drawn on by this GPU"))?;
        // Not an sRGB format, as on the desktop: encoded colours blend the
        // way a browser blends them.
        let plain = config.format.remove_srgb_suffix();
        if surface.get_capabilities(&adapter).formats.contains(&plain) {
            config.format = plain;
        }
        let format = config.format;
        surface.configure(&device, &config);
        let compositor = Compositor::new(device.clone(), queue.clone(), format);
        let access = if can_comment {
            Access::Comment
        } else {
            Access::View
        };
        let mut viewer = Viewer::new(&canvas_text, access, seed.to_bits()).map_err(error)?;
        viewer.set_text_measure(Arc::new(compositor.text_measure()));
        Ok(Self {
            viewer,
            gpu: Gpu {
                surface,
                device,
                queue,
                config,
                compositor,
            },
            cache: ViewCache::default(),
            viewport: Vec2::ZERO,
            scale: 1.0,
            drawn_zoom: 0.0,
        })
    }

    /// The canvas element is `width` by `height` CSS pixels at `scale`
    /// device pixels each.
    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.viewport = Vec2::new(width, height);
        self.scale = scale;
        let gpu = &mut self.gpu;
        gpu.config.width = ((width * scale).round() as u32).max(1);
        gpu.config.height = ((height * scale).round() as u32).max(1);
        gpu.surface.configure(&gpu.device, &gpu.config);
        self.viewer.resize(self.viewport, scale);
    }

    /// A published image or markdown file, by the path the canvas writes.
    pub fn add_file(&mut self, path: &str, bytes: &Uint8Array) {
        self.viewer.add_file(path, bytes.to_vec());
    }

    /// Every published file has been given.
    pub fn files_complete(&mut self) {
        self.viewer.files_complete();
    }

    /// A page's snapshot PNG.
    pub fn add_snapshot(&mut self, page: &str, png: &Uint8Array) {
        self.viewer.add_snapshot(page, png.to_vec());
    }

    /// The svgs to draw since the last call, as `{ image, file, width,
    /// height }`: `image` is answered to [`WebViewer::svg_drawn`], and the
    /// width and height in device pixels are absent for the svg's own size.
    pub fn take_svg_jobs(&mut self) -> js_sys::Array {
        (self.viewer.take_svg_jobs().into_iter())
            .map(|job| {
                let object = js_sys::Object::new();
                let set = |key: &str, value: JsValue| {
                    let _ = js_sys::Reflect::set(&object, &key.into(), &value);
                };
                set("image", JsValue::from(job.image.0));
                set("file", job.file.into());
                if let Some(want) = job.want {
                    set("width", want.width.into());
                    set("height", want.height.into());
                }
                object
            })
            .collect()
    }

    /// The svg `image` drawn into `width` by `height` pixels of RGBA, at
    /// its own size `intrinsic_width` by `intrinsic_height`.
    pub fn svg_drawn(
        &mut self,
        image: u64,
        intrinsic_width: u32,
        intrinsic_height: u32,
        width: u32,
        height: u32,
        rgba: &Uint8Array,
    ) {
        self.viewer.svg_drawn(
            ImageKey(image),
            PixelSize::new(intrinsic_width, intrinsic_height),
            PixelSize::new(width, height),
            rgba.to_vec(),
        );
    }

    /// The svg `image` could not be drawn.
    pub fn svg_failed(&mut self, image: u64) {
        self.viewer.svg_failed(ImageKey(image));
    }

    /// Two fingers on a touch screen, in CSS pixels: their midpoint is at
    /// (`x`, `y`) and moved by (`dx`, `dy`), and they are `spread` times as
    /// far apart as they were.
    pub fn touch_gesture(&mut self, x: f32, y: f32, dx: f32, dy: f32, spread: f32) {
        self.viewer
            .touch_gesture(Vec2::new(x, y), Vec2::new(dx, dy), spread);
    }

    /// A pointer event: `kind` is `move`, `down`, `up` or `leave`, at CSS
    /// pixels from the canvas element's top-left.
    pub fn pointer(&mut self, kind: &str, x: f32, y: f32, button_index: u8, mods: u8) {
        let button = button(button_index);
        let kind = match kind {
            "down" => PointerEventKind::Down {
                button,
                click_count: 1,
            },
            "up" => PointerEventKind::Up {
                button,
                click_count: 1,
            },
            "leave" => PointerEventKind::Leave,
            _ => PointerEventKind::Move,
        };
        self.viewer.pointer(kind, Vec2::new(x, y), modifiers(mods));
    }

    /// A wheel event's `deltaX` and `deltaY`, in CSS pixels. The DOM's
    /// positive `deltaY` scrolls down, moving content up, the opposite of
    /// the app's.
    pub fn wheel(&mut self, dx: f32, dy: f32, mods: u8) {
        self.viewer.wheel(-Vec2::new(dx, dy), modifiers(mods));
    }

    /// A key event. Returns whether the viewer used it, so the page can
    /// call `preventDefault`.
    pub fn key(&mut self, code: &str, key: &str, pressed: bool, repeat: bool, mods: u8) -> bool {
        self.viewer
            .key(key_input(code, key, pressed, repeat, modifiers(mods)))
    }

    /// `pan` or `comment`. Returns whether the mode is now the one asked for.
    pub fn set_mode(&mut self, mode: &str) -> bool {
        let mode = if mode == "comment" {
            Mode::Comment
        } else {
            Mode::Pan
        };
        self.viewer.set_mode(mode)
    }

    /// The CSS cursor for the canvas element.
    pub fn cursor(&self) -> String {
        use specular_core::Cursor;
        match self.viewer.cursor() {
            Cursor::Crosshair => "crosshair",
            Cursor::Grab => "grab",
            Cursor::Grabbing => "grabbing",
            Cursor::Text | Cursor::VerticalText => "text",
            Cursor::Pointer => "pointer",
            _ => "default",
        }
        .to_owned()
    }

    /// The live room's whole state: the first frame of a connection.
    pub fn live_state(&mut self, state: &Uint8Array) {
        self.viewer.live_state(state.to_vec());
    }

    /// One update from the live room.
    pub fn live_update(&mut self, update: &Uint8Array) {
        self.viewer.live_update(update.to_vec());
    }

    /// Whether the canvas shown is the live one.
    pub fn is_live(&self) -> bool {
        self.viewer.is_live()
    }

    /// The server's comments, as a JSON array of `.canvas` annotations.
    pub fn set_comments(&mut self, json: &str) -> Result<(), JsValue> {
        let comments = serde_json::from_str(json).map_err(error)?;
        self.viewer.set_comments(comments);
        Ok(())
    }

    /// The comments made here since the last call, as a JSON array, for the
    /// page to send.
    pub fn take_new_comments(&mut self) -> Result<String, JsValue> {
        serde_json::to_string(&self.viewer.take_new_comments()).map_err(error)
    }

    /// Every comment on the canvas as it is drawn, as a JSON array of
    /// `.canvas` annotations: the published ones, the server's and the ones
    /// made here.
    pub fn comments(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.viewer.app().document().annotations()).map_err(error)
    }

    /// The id of the comment that has the focus.
    pub fn focused_comment(&self) -> Option<String> {
        self.viewer
            .focused_comment()
            .map(|id| id.as_str().to_owned())
    }

    /// Gives a comment the focus, or with no id takes it away.
    pub fn focus_comment(&mut self, id: Option<String>) {
        self.viewer.focus_comment(id.map(AnnotationId::new));
    }

    /// Brings a comment into view and gives it the focus.
    pub fn reveal_comment(&mut self, id: String) {
        self.viewer.reveal_comment(AnnotationId::new(id));
    }

    /// Fits the whole canvas in the view.
    pub fn zoom_to_fit(&mut self) {
        self.viewer.zoom_to_fit();
    }

    /// One animation frame at `now_ms` (`Date.now()`). Draws only when
    /// something changed. Returns whether it drew.
    pub fn frame(&mut self, now_ms: f64) -> Result<bool, JsValue> {
        self.viewer.tick(now_ms as u64);
        self.upload()?;
        let zoom = self.viewer.app().session().camera.zoom;
        let zooming = (zoom - self.drawn_zoom).abs() > f32::EPSILON;
        if !self.viewer.take_dirty() && !zooming {
            return Ok(false);
        }
        self.drawn_zoom = zoom;
        self.draw(zooming);
        // A zoom that just stopped is drawn once more with sharp text.
        if zooming {
            self.draw(false);
        }
        Ok(true)
    }
}

impl WebViewer {
    /// Hands the compositor what the viewer has for it.
    fn upload(&mut self) -> Result<(), JsValue> {
        let compositor = &mut self.gpu.compositor;
        for upload in self.viewer.take_uploads() {
            match upload {
                Upload::PageFrame {
                    page,
                    viewport,
                    frame,
                } => compositor
                    .handle_page_event(PageEvent::Frame(FrameEvent {
                        page,
                        layer: FrameLayer::View,
                        viewport,
                        frame: PageFrame::Cpu(frame),
                        produced_at: Instant::now(),
                    }))
                    .map_err(error)?,
                Upload::ClosePage(page) => compositor.remove_page(page),
                Upload::Image { key, size, rgba } => compositor
                    .set_image(ImageId(key.texture(0)), size, &rgba)
                    .map_err(error)?,
                Upload::DropImage(key) => compositor.remove_image(ImageId(key.texture(0))),
            }
        }
        Ok(())
    }

    fn draw(&mut self, zooming: bool) {
        let app = self.viewer.app();
        let scene = specular_scene::view(app, self.viewport, &self.cache);
        let frame = FrameView {
            camera: app.session().camera,
            viewport: self.viewport,
            scale_factor: self.scale,
            grid: DotGrid::themed(scene.colors()),
            zooming,
        };
        let gpu = &mut self.gpu;
        let (wgpu::CurrentSurfaceTexture::Success(texture)
        | wgpu::CurrentSurfaceTexture::Suboptimal(texture)) = gpu.surface.get_current_texture()
        else {
            gpu.surface.configure(&gpu.device, &gpu.config);
            return;
        };
        let target = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let viewer = &self.viewer;
        gpu.compositor
            .render_scene(&target, &frame, &scene, |entity| viewer.page_of(entity));
        gpu.queue.present(texture);
    }
}
