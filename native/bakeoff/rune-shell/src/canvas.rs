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
    pages: String,
}

/// With `RUNE_SHELL_PACE=1`: the frames of one burst (frames less than
/// 100 ms apart), printed when a gap ends it, with the CPU time our
/// `render` took over them.
#[derive(Debug, Default)]
struct Pace {
    last: Option<std::time::Instant>,
    intervals: Vec<f64>,
    cpu: Vec<f64>,
    update: Vec<f64>,
}

impl Pace {
    fn frame(
        &mut self,
        now: std::time::Instant,
        (cpu_ms, update_ms): (f64, f64),
        size: (u32, u32),
    ) {
        if let Some(last) = self.last {
            let gap = now.duration_since(last).as_secs_f64() * 1e3;
            if gap > 100.0 {
                self.report(size, gap);
            } else {
                self.intervals.push(gap);
            }
        }
        self.last = Some(now);
        self.cpu.push(cpu_ms);
        self.update.push(update_ms);
    }

    fn report(&mut self, size: (u32, u32), gap: f64) {
        let mut intervals = std::mem::take(&mut self.intervals);
        let cpu = std::mem::take(&mut self.cpu);
        let update = std::mem::take(&mut self.update);
        let mean_update = update.iter().sum::<f64>() / update.len().max(1) as f64;
        intervals.sort_by(f64::total_cmp);
        let at = |q: f64| intervals.get(((intervals.len() as f64 - 1.0) * q).round() as usize);
        let mean_cpu = cpu.iter().sum::<f64>() / cpu.len().max(1) as f64;
        eprintln!(
            "rune-shell: burst {} frames, interval p50 {:.2} ms p95 {:.2} ms max {:.2} ms, \
             render cpu mean {:.2} ms, rune update mean {:.2} ms, size {:?}, then idle {:.0} ms",
            cpu.len(),
            at(0.5).copied().unwrap_or(0.0),
            at(0.95).copied().unwrap_or(0.0),
            intervals.last().copied().unwrap_or(0.0),
            mean_cpu,
            mean_update,
            size,
            gap,
        );
    }
}

/// The canvas under Rune's UI.
pub struct Canvas {
    pace: Option<Pace>,
    /// Samples per pixel of our scene pass.
    samples: u32,
    host: Option<Host>,
    seen: Seen,
    /// Whether the last frame changed something the next one shows.
    owes_frame: bool,
    /// The window is closing and the compositor holding page surfaces has
    /// been let go of.
    released: bool,
    /// `RUNE_SHELL_GPU_TIME=1` (S6): wait for the GPU before and after our
    /// pass, and time it from its submit to done. It stalls each frame on
    /// the GPU, so its frame intervals are not the shell's.
    gpu_time: bool,
}

impl Canvas {
    /// A canvas with no compositor yet: Rune's device comes with the first
    /// frame. `samples` is 4, or 1 on a backend with no multisampling.
    pub fn new(samples: u32) -> Self {
        Self {
            samples,
            pace: std::env::var_os("RUNE_SHELL_PACE").map(|_| Pace::default()),
            host: None,
            seen: Seen::default(),
            owes_frame: false,
            released: false,
            gpu_time: std::env::var_os("RUNE_SHELL_GPU_TIME").is_some(),
        }
    }

    /// The compositor for this frame's device and format, made again when
    /// either changes.
    fn host_for(&mut self, f: &Frame<'_, Shell>) -> &mut Host {
        let key = f.device_key();
        let stale =
            (self.host.as_ref()).is_none_or(|host| host.key != key || host.format != f.format);
        if stale {
            let mut compositor =
                Compositor::with_samples(f.device.clone(), f.queue.clone(), f.format, self.samples);
            compositor.warm_text();
            self.seen.devices += 1;
            self.seen.format = Some(f.format);
            self.seen.adapter = f.adapter().map_or_else(String::new, |adapter| {
                let info = adapter.get_info();
                format!("{} ({:?})", info.name, info.backend)
            });
            // What S5's IOSurface import asks of the device.
            #[cfg(target_os = "macos")]
            // SAFETY: the guard is dropped at once; nothing is done through it.
            let metal_hal = unsafe { f.device.as_hal::<wgpu::hal::api::Metal>() }.is_some();
            #[cfg(not(target_os = "macos"))]
            let metal_hal = false;
            eprintln!(
                "rune-shell: compositor on {} target {:?} srgb_blend {} {} samples size {:?}, \
                 as_hal::<Metal> {}",
                self.seen.adapter,
                f.format,
                f.srgb_blend(),
                self.samples,
                f.size,
                if metal_hal { "answers" } else { "none" },
            );
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
    if shell.closing.is_some() {
        shell.page_events.clear();
    }
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

/// How page frames came: zero-copy surfaces or CPU bytes, and how long a
/// frame waited between CEF's paint and the poll that took it.
fn page_line(shell: &Shell) -> String {
    let tally = shell.pages.tally;
    let count = (tally.shared + tally.cpu).max(1);
    #[cfg(feature = "cef")]
    let wakes = crate::cef::wakes();
    #[cfg(not(feature = "cef"))]
    let wakes = 0;
    format!(
        "pages from {}: {} shared (IOSurface) frames, {} cpu frames, paint to poll mean {:.2} ms \
         max {:.2} ms, {} wakes on paint",
        shell.pages.name,
        tally.shared,
        tally.cpu,
        tally.delay_us as f64 / count as f64 / 1e3,
        tally.max_delay_us as f64 / 1e3,
        wakes,
    )
}

impl RuneCompositor<Shell> for Canvas {
    fn render(&mut self, f: &mut Frame<'_, Shell>) {
        let started = std::time::Instant::now();
        let traced_at = specular_core::ledger::now_us();
        let scale = f.rt.window_scale();
        let viewport = Vec2::new(f.size.0 as f32 / scale, f.size.1 as f32 / scale);
        self.seen.srgb_blend = f.srgb_blend();
        // CEF stops slowly, and a run that ends with it stopping hit
        // Chromium's teardown watchdog, while the compositor still holds
        // page surfaces. A closing window hands them back first; the pages
        // draw as placeholders until it goes.
        if f.rt.state.closing.is_some() && !self.released {
            self.released = true;
            self.host = None;
        }
        let gpu_time = self.gpu_time;
        let host = self.host_for(f);

        let shell = &mut f.rt.state;
        shell.resized(viewport, scale);
        sync(host, shell);
        if let Some(mut bench) = shell.bench.take() {
            bench.before_frame(shell, started);
            shell.bench = Some(bench);
        }
        let scene = shell.scene();
        let frame = FrameView {
            camera: shell.app.session().camera,
            viewport,
            scale_factor: scale,
            grid: DotGrid::themed(scene.colors()),
            zooming: shell.app.zoom_moving(),
        };
        let wait = wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        };
        if gpu_time {
            let _ = f.device.poll(wait.clone());
        }
        let hosts = &shell.hosts;
        let stats = host
            .compositor
            .render_scene(f.target, &frame, &scene, |entity: &EntityId| {
                hosts.get(entity).copied()
            });
        if gpu_time {
            let submitted = std::time::Instant::now();
            let _ = f.device.poll(wait);
            let ms = submitted.elapsed().as_secs_f64() * 1e3;
            if let Some(bench) = shell.bench.as_mut() {
                bench.gpu(ms);
            }
        }
        // A Document's rows are measured by drawing them, and their heights
        // move what is below: the next frame shows it.
        let heights = host.compositor.column_heights().to_vec();
        let moved = shell.report_note_heights(&heights);
        let pages = page_line(shell);
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

        let render_ms = started.elapsed().as_secs_f64() * 1e3;
        if specular_core::ledger::enabled() {
            let size = specular_core::PixelSize::new(f.size.0, f.size.1);
            specular_core::ledger::frame(traced_at, size, false);
        }
        let pacing =
            (f.rt.surface_stats()).map(|stats| (stats.pacing.to_string(), stats.totals.presents));
        let update_ms = f.update_ms;
        f.rt.state.latency.presented();
        let benching = match f.rt.state.bench.as_mut() {
            Some(bench) => {
                bench.after_frame(started, &stats, (render_ms, update_ms), pacing);
                bench.animating(std::time::Instant::now())
            }
            None => false,
        };
        self.owes_frame = moved || stats.text.settling || frame.zooming || benching;
        if let Some(pace) = &mut self.pace {
            pace.frame(
                started,
                (started.elapsed().as_secs_f64() * 1e3, f.update_ms),
                f.size,
            );
        }
        self.seen.frames += 1;
        self.seen.stats = stats;
        self.seen.state = state;
        self.seen.pages = pages;
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
        if let Some(pace) = &mut self.pace {
            pace.report((0, 0), 0.0);
        }
        // The run is over. Page surfaces go back to CEF before the shell,
        // which Rune drops first, stops it.
        self.host = None;
        let seen = &self.seen;
        Some(format!(
            "specular: {} frames on {} device(s), adapter {}, target {:?}, srgb_blend {}, \
             {} samples\n\
             specular: last frame drew {} items in {} draw calls, {} glyphs, {} triangles, \
             {} pages without a texture\n\
             specular: {}\n\
             specular: {}",
            seen.frames,
            seen.devices,
            seen.adapter,
            seen.format,
            seen.srgb_blend,
            self.samples,
            seen.stats.items_drawn,
            seen.stats.draw_calls,
            seen.stats.glyphs,
            seen.stats.triangles,
            seen.stats.render.pages_without_texture,
            seen.state,
            seen.pages,
        ))
    }
}
