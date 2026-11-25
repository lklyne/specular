//! [`Canvas`]: the Rune `Compositor` that hosts `specular-compositor`.
//!
//! Each frame it draws the app's scene into Rune's target with our
//! compositor, made on Rune's device, then draws Rune's UI over it.
//!
//! Two things are not as ADR 0051 assumed:
//!
//! - `Compositor::render_scene` makes its own command encoder and submits
//!   it. That lands ahead of Rune's encoder, which is submitted after
//!   `render` returns, so the order on the GPU is canvas then UI. It costs
//!   a second submit a frame.
//! - `Frame::draw_ui` clears the target, so it cannot follow our pass.
//!   The UI is drawn with the pieces `draw_ui` is made of, loading what the
//!   target holds.

use std::collections::HashMap;

use glam::Vec2;
use rune::compose::{Compositor as RuneCompositor, DeviceKey, Frame};
use specular_compositor::{Compositor, DotGrid, FrameView, ImageMips, SceneStats};
use specular_doc::EntityId;
use specular_interact::ImageKey;
use specular_scene::ImageId;

use crate::shell::Shell;

/// Our compositor and what it has been given, all of it made on one device.
struct Host {
    key: DeviceKey,
    format: wgpu::TextureFormat,
    compositor: Compositor,
    /// The serial of each image uploaded, and how many frames it has.
    uploaded: HashMap<ImageKey, (u64, u32)>,
}

/// What the frames so far did, for the line printed after a headless run.
#[derive(Debug, Default)]
struct Seen {
    frames: u64,
    devices: u32,
    format: Option<wgpu::TextureFormat>,
    srgb_blend: bool,
    adapter: String,
    stats: SceneStats,
    state: String,
}

/// The canvas under Rune's UI.
#[derive(Default)]
pub struct Canvas {
    host: Option<Host>,
    seen: Seen,
    /// Whether the last frame changed something the next one shows.
    owes_frame: bool,
}

impl Canvas {
    /// A canvas with no compositor yet: Rune's device comes with the first
    /// frame.
    pub fn new() -> Self {
        Self::default()
    }

    /// The compositor for this frame's device and format, made again when
    /// either changes.
    fn host_for(&mut self, f: &Frame<'_, Shell>) -> &mut Host {
        let key = f.device_key();
        let stale =
            (self.host.as_ref()).is_none_or(|host| host.key != key || host.format != f.format);
        if stale {
            let mut compositor = Compositor::new(f.device.clone(), f.queue.clone(), f.format);
            compositor.warm_text();
            self.seen.devices += 1;
            self.seen.format = Some(f.format);
            self.seen.adapter = f.adapter().map_or_else(String::new, |adapter| {
                let info = adapter.get_info();
                format!("{} ({:?})", info.name, info.backend)
            });
            self.host = Some(Host {
                key,
                format: f.format,
                compositor,
                uploaded: HashMap::new(),
            });
        }
        self.host.get_or_insert_with(|| unreachable!("set above"))
    }
}

/// Gives `host` what the shell gathered since the last frame: page frames,
/// closed pages, and images decoded, redrawn or let go of.
fn sync(host: &mut Host, shell: &mut Shell) {
    for event in shell.page_events.drain(..) {
        if let Err(error) = host.compositor.handle_page_event(event) {
            eprintln!("rune-shell: {error}");
        }
    }
    for page in shell.closed_pages.drain(..) {
        host.compositor.remove_page(page);
    }
    let spec = host.compositor.image_spec();
    for (key, held) in &shell.images {
        let before = host.uploaded.get(key).copied();
        if before.is_some_and(|(serial, _)| serial == held.serial) {
            continue;
        }
        let frames = held.decoded.frames.len() as u32;
        for (frame, pixels) in held.decoded.frames.iter().enumerate() {
            let id = ImageId(key.texture(frame as u32));
            let put = ImageMips::build(pixels.size, &pixels.rgba, spec)
                .and_then(|mips| host.compositor.set_image_mips(id, &mips));
            if let Err(error) = put {
                eprintln!("rune-shell: image not uploaded: {error}");
            }
        }
        for frame in frames..before.map_or(0, |(_, frames)| frames) {
            host.compositor.remove_image(ImageId(key.texture(frame)));
        }
        host.uploaded.insert(*key, (held.serial, frames));
    }
    let gone: Vec<_> = (host.uploaded.keys())
        .filter(|key| !shell.images.contains_key(key))
        .copied()
        .collect();
    for key in gone {
        let (_, frames) = host.uploaded.remove(&key).unwrap_or_default();
        for frame in 0..frames {
            host.compositor.remove_image(ImageId(key.texture(frame)));
        }
    }
}

impl RuneCompositor<Shell> for Canvas {
    fn render(&mut self, f: &mut Frame<'_, Shell>) {
        let scale = f.rt.window_scale();
        let viewport = Vec2::new(f.size.0 as f32 / scale, f.size.1 as f32 / scale);
        self.seen.srgb_blend = f.srgb_blend();
        let host = self.host_for(f);

        let shell = &mut f.rt.state;
        shell.resized(viewport, scale);
        sync(host, shell);
        let scene = shell.scene();
        let frame = FrameView {
            camera: shell.app.session().camera,
            viewport,
            scale_factor: scale,
            grid: DotGrid::themed(scene.colors()),
            zooming: shell.app.zoom_moving(),
        };
        let hosts = &shell.hosts;
        let stats = host
            .compositor
            .render_scene(f.target, &frame, &scene, |entity: &EntityId| {
                hosts.get(entity).copied()
            });
        // A Document's rows are measured by drawing them, and their heights
        // move what is below: the next frame shows it.
        let heights = host.compositor.column_heights().to_vec();
        let moved = shell.report_note_heights(&heights);
        let state = format!(
            "tool={:?} selection={:?} entities={}",
            shell.app.session().tool,
            shell.app.session().selection,
            shell.app.document().entities().count(),
        );

        // Rune's UI over the canvas: `Frame::draw_ui` with `LoadOp::Load`
        // where it clears.
        f.ui.set_fade_stops(f.rt.fade_stops());
        f.ui.prepare(
            f.device,
            f.queue,
            &mut f.rt.text.atlas,
            &f.rt.instances,
            f.size,
        );
        let count = f.rt.instances.len() as u32;
        f.ui.encode_layers(f.encoder);
        f.ui.draw_target_range_with(
            f.encoder,
            f.target,
            f.format,
            wgpu::LoadOp::Load,
            0..count,
            &mut |_, _| {},
        );

        self.owes_frame = moved || stats.text.settling || frame.zooming;
        self.seen.frames += 1;
        self.seen.stats = stats;
        self.seen.state = state;
    }

    fn release_gpu(&mut self) {
        // Everything of ours on the lost device goes with it. Images are
        // uploaded again from the shell's pixels; a page has no texture
        // until it next paints.
        self.host = None;
    }

    fn animating(&self) -> bool {
        self.owes_frame
    }

    fn summary(&mut self, _: &wgpu::Device) -> Option<String> {
        let seen = &self.seen;
        Some(format!(
            "specular: {} frames on {} device(s), adapter {}, target {:?}, srgb_blend {}\n\
             specular: last frame drew {} items in {} draw calls, {} glyphs, {} triangles, \
             {} pages without a texture\n\
             specular: {}",
            seen.frames,
            seen.devices,
            seen.adapter,
            seen.format,
            seen.srgb_blend,
            seen.stats.items_drawn,
            seen.stats.draw_calls,
            seen.stats.glyphs,
            seen.stats.triangles,
            seen.stats.render.pages_without_texture,
            seen.state,
        ))
    }
}
