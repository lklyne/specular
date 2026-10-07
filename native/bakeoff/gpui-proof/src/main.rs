//! Throwaway gpui bake-off proof: zoomed canvas of sticky notes + strokes + toolbar.
//! Env: SURFACE=bgra|nv12 paints one IOSurface-backed CVPixelBuffer with gpui's `surface`
//! element; LOD=1 skips note text below 4px; PAINT=1 draws notes from one canvas element
//! (paint_quad + shape_text) instead of one div per note.
use core_foundation::{
    base::{TCFType, kCFAllocatorDefault},
    dictionary::CFDictionary,
    number::CFNumber,
    string::CFString,
};
use core_video::{
    pixel_buffer::{
        CVPixelBuffer, kCVPixelFormatType_32BGRA, kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
    },
    pixel_buffer_io_surface::CVPixelBufferCreateWithIOSurface,
};
use gpui::{
    App, Application, Bounds, Context, PathBuilder, SharedString, TextRun, Window,
    WindowBounds, WindowOptions, canvas, div, font, point, prelude::*, px, quad, rgb, size,
    surface,
};
use std::{cell::RefCell, rc::Rc, time::Instant};

const NOTES: usize = 500;
const COLS: usize = 25;
const TEXT: &str = "Sticky note with enough wrapped text to span several lines at zoom one";

#[derive(Default)]
struct Stats {
    last_frame: Option<Instant>,
    intervals: Vec<f64>,
    render: Vec<f64>,
    to_paint_end: Vec<f64>,
    to_present_end: Vec<f64>,
    visible: Vec<f64>,
    frame_start: Option<Instant>,
}

fn summary(label: &str, v: &mut [f64]) {
    if v.is_empty() {
        return println!("{label}: no samples");
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let p95 = v[(v.len() as f64 * 0.95) as usize];
    println!(
        "{label}: n={} mean={mean:.2} p95={p95:.2} max={:.2}",
        v.len(),
        v[v.len() - 1]
    );
}

struct Proof {
    start: Instant,
    stats: Rc<RefCell<Stats>>,
    reported: bool,
    surface: Option<CVPixelBuffer>,
    lod: bool,
    paint_mode: bool,
    clicks: usize,
}

/// One IOSurface with a test pattern, wrapped in a CVPixelBuffer (what a CEF page would hand us).
fn make_surface(kind: &str) -> CVPixelBuffer {
    let (w, h) = (512usize, 320usize);
    let bgra = kind == "bgra";
    let fourcc = if bgra {
        kCVPixelFormatType_32BGRA
    } else {
        kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
    };
    if !bgra {
        // Planar IOSurfaces need per-plane properties; let CoreVideo allocate the IOSurface.
        let empty: CFDictionary<CFString, core_foundation::base::CFType> =
            CFDictionary::from_CFType_pairs(&[]);
        let key = CFString::from_static_string("IOSurfaceProperties");
        let opts = CFDictionary::from_CFType_pairs(&[(key, empty.as_CFType())]);
        let buf = CVPixelBuffer::new(fourcc, w, h, Some(&opts)).expect("CVPixelBufferCreate");
        buf.lock_base_address(0);
        unsafe {
            for plane in 0..2 {
                let base = buf.get_base_address_of_plane(plane) as *mut u8;
                let stride = buf.get_bytes_per_row_of_plane(plane);
                let ph = buf.get_height_of_plane(plane);
                for y in 0..ph {
                    for x in 0..stride {
                        let v = if plane == 0 {
                            if (x / 32 + y / 32) % 2 == 0 { 220 } else { 60 }
                        } else if x % 2 == 0 {
                            90
                        } else {
                            200
                        };
                        *base.add(y * stride + x) = v;
                    }
                }
            }
        }
        buf.unlock_base_address(0);
        return buf;
    }
    let n = |v: usize| CFNumber::from(v as i64).as_CFType();
    let k = CFString::from_static_string;
    let props = CFDictionary::from_CFType_pairs(&[
        (k("IOSurfaceWidth"), n(w)),
        (k("IOSurfaceHeight"), n(h)),
        (k("IOSurfaceBytesPerElement"), n(4)),
        (k("IOSurfacePixelFormat"), n(fourcc as usize)),
    ]);
    let io = io_surface::new(&props);
    unsafe {
        use io_surface::{IOSurfaceGetBaseAddress, IOSurfaceGetBytesPerRow, IOSurfaceLock};
        let r = io.as_concrete_TypeRef();
        IOSurfaceLock(r, 0, std::ptr::null_mut());
        let base = IOSurfaceGetBaseAddress(r) as *mut u8;
        let stride = IOSurfaceGetBytesPerRow(r);
        for y in 0..h {
            for x in 0..w {
                let on = (x / 32 + y / 32) % 2 == 0;
                let px = base.add(y * stride + x * 4);
                *px = if on { 255 } else { 30 }; // B
                *px.add(1) = (x * 255 / w) as u8; // G
                *px.add(2) = if on { 30 } else { 255 }; // R
                *px.add(3) = 255;
            }
        }
        io_surface::IOSurfaceUnlock(r, 0, std::ptr::null_mut());
        let mut out = std::ptr::null_mut();
        let rc = CVPixelBufferCreateWithIOSurface(kCFAllocatorDefault, r, std::ptr::null(), &mut out);
        assert_eq!(rc, 0, "CVPixelBufferCreateWithIOSurface");
        CVPixelBuffer::wrap_under_create_rule(out)
    }
}

impl Render for Proof {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        let t = now.duration_since(self.start).as_secs_f32();
        if t > 8.0 {
            cx.quit();
        }
        let measuring = (1.0..6.0).contains(&t);
        if t >= 6.0 && !self.reported {
            self.reported = true;
            let mut s = self.stats.borrow_mut();
            println!("--- gpui 0.2.2 proof, {NOTES} notes, ms over ~5 s ---");
            summary("frame interval      ", &mut s.intervals);
            summary("render() build      ", &mut s.render);
            summary("render..paint end   ", &mut s.to_paint_end);
            summary("render..present end ", &mut s.to_present_end);
            summary("visible notes       ", &mut s.visible);
        }
        window.request_animation_frame();

        // Zoom sweeps 0.05..3 in log space, one triangle period per 5 s.
        let phase = (t / 2.5) % 2.0;
        let tri = if phase < 1.0 { phase } else { 2.0 - phase };
        let zoom = (0.05f32.ln() + (3.0f32.ln() - 0.05f32.ln()) * tri).exp();
        let vp = window.viewport_size();
        let (vw, vh) = (f32::from(vp.width), f32::from(vp.height));
        let (cx_w, cy_w) = (COLS as f32 * 120.0, (NOTES / COLS) as f32 * 90.0);
        let to_screen =
            move |x: f32, y: f32| ((x - cx_w) * zoom + vw / 2.0, (y - cy_w) * zoom + vh / 2.0);

        let font_px = 14.0 * zoom;
        let draw_text = !(self.lod && font_px < 4.0);
        let mut rects = Vec::new();
        for i in 0..NOTES {
            let (wx, wy) = ((i % COLS) as f32 * 240.0, (i / COLS) as f32 * 180.0);
            let (sx, sy) = to_screen(wx, wy);
            let (w, h) = (200.0 * zoom, 140.0 * zoom);
            if sx + w < 0.0 || sy + h < 0.0 || sx > vw || sy > vh {
                continue; // viewport cull
            }
            rects.push((i, sx, sy, w, h));
        }
        let visible = rects.len();

        let mut layer = div().absolute().size_full().overflow_hidden();
        if self.paint_mode {
            layer = layer.child(
                canvas(
                    |_, _, _| {},
                    move |_, _, window, cx| {
                        for (i, sx, sy, w, h) in rects {
                            let b = Bounds::new(point(px(sx), px(sy)), size(px(w), px(h)));
                            let fill = rgb(if i % 2 == 0 { 0xfde68a } else { 0xbfdbfe });
                            window.paint_quad(quad(b, px(8.0 * zoom), fill, px(0.), fill, Default::default()));
                            if !draw_text {
                                continue;
                            }
                            let run = TextRun {
                                len: TEXT.len(),
                                font: font(".SystemUIFont"),
                                color: rgb(0x1f2937).into(),
                                background_color: None,
                                underline: None,
                                strikethrough: None,
                            };
                            let pad = 10.0 * zoom;
                            let lines = window
                                .text_system()
                                .shape_text(TEXT.into(), px(font_px), &[run], Some(px(w - 2.0 * pad)), None)
                                .unwrap();
                            let lh = px(font_px * 1.3);
                            for line in lines {
                                let o = point(px(sx + pad), px(sy + pad));
                                line.paint(o, lh, gpui::TextAlign::Left, None, window, cx).ok();
                            }
                        }
                    },
                )
                .absolute()
                .size_full(),
            );
        } else {
            for (i, sx, sy, w, h) in rects {
                let mut note = div()
                    .absolute()
                    .left(px(sx))
                    .top(px(sy))
                    .w(px(w))
                    .h(px(h))
                    .p(px(10.0 * zoom))
                    .rounded(px(8.0 * zoom))
                    .bg(rgb(if i % 2 == 0 { 0xfde68a } else { 0xbfdbfe }))
                    .overflow_hidden()
                    .text_size(px(font_px))
                    .line_height(px(font_px * 1.3))
                    .text_color(rgb(0x1f2937));
                if draw_text {
                    note = note.child(SharedString::new_static(TEXT));
                }
                layer = layer.child(note);
            }
        }

        // 40 freehand strokes + arrows, rebuilt and tessellated (lyon) every frame at screen scale.
        let strokes = canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                for s in 0..40 {
                    let mut b = PathBuilder::stroke(px((3.0 * zoom).max(1.0)));
                    for j in 0..32 {
                        let wx = s as f32 * 150.0 + j as f32 * 20.0;
                        let wy = 400.0 + s as f32 * 80.0 + (j as f32 * 0.6 + s as f32).sin() * 60.0;
                        let (x, y) = to_screen(wx, wy);
                        if j == 0 { b.move_to(point(px(x), px(y))) } else { b.line_to(point(px(x), px(y))) }
                    }
                    if let Ok(path) = b.build() {
                        window.paint_path(path, rgb(if s % 2 == 0 { 0xdc2626 } else { 0x2563eb }));
                    }
                }
            },
        )
        .absolute()
        .size_full();

        let mut toolbar = div()
            .absolute()
            .top(px(8.))
            .left(px(8.))
            .flex()
            .gap(px(6.))
            .p(px(6.))
            .rounded(px(8.))
            .bg(rgb(0x111827));
        for (i, name) in ["Select", "Hand", "Page", "Note", "Draw", "Arrow", "Text", "Zoom"].iter().enumerate() {
            toolbar = toolbar.child(
                div()
                    .id(("tool", i))
                    .px(px(10.))
                    .py(px(4.))
                    .rounded(px(6.))
                    .bg(rgb(0x374151))
                    .hover(|s| s.bg(rgb(0x4b5563)))
                    .text_color(rgb(0xffffff))
                    .text_size(px(13.))
                    .child(*name)
                    .on_click(cx.listener(|this, _, _, _| this.clicks += 1)),
            );
        }
        toolbar = toolbar.child(
            div().text_color(rgb(0x9ca3af)).text_size(px(13.)).child(format!("zoom {zoom:.2}  notes {visible}")),
        );

        let stats = self.stats.clone();
        let render_ms = now.elapsed().as_secs_f64() * 1e3;
        {
            let mut s = stats.borrow_mut();
            if let (true, Some(last)) = (measuring, s.last_frame) {
                s.intervals.push(now.duration_since(last).as_secs_f64() * 1e3);
                s.render.push(render_ms);
                s.visible.push(visible as f64);
            }
            s.last_frame = Some(now);
            s.frame_start = Some(now);
        }
        // Painted last, so its paint closure marks the end of layout + prepaint + paint.
        let paint_end = {
            let stats = stats.clone();
            canvas(|_, _, _| {}, move |_, _, _, _| {
                let mut s = stats.borrow_mut();
                if let (true, Some(t0)) = (measuring, s.frame_start) {
                    s.to_paint_end.push(t0.elapsed().as_secs_f64() * 1e3);
                }
            })
        };
        // Deferred effects flush when the frame's outermost update ends, i.e. after
        // `window.draw` + `window.present` (Metal encode + commit) in gpui's frame callback.
        cx.defer(move |_| {
            let mut s = stats.borrow_mut();
            if let (true, Some(t0)) = (measuring, s.frame_start) {
                s.to_present_end.push(t0.elapsed().as_secs_f64() * 1e3);
            }
        });

        let mut root = div().size_full().relative().bg(rgb(0xf3f4f6)).child(layer).child(strokes);
        if let Some(buf) = &self.surface {
            root = root.child(surface(buf.clone()).absolute().left(px(40.)).top(px(80.)).w(px(512.)).h(px(320.)));
        }
        root.child(toolbar).child(paint_end)
    }
}

fn main() {
    // Safety net: never leave a window open even if the frame loop stalls.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(14));
        eprintln!("watchdog exit");
        std::process::exit(2);
    });
    let surface = std::env::var("SURFACE").ok().map(|k| make_surface(&k));
    let lod = std::env::var("LOD").is_ok();
    let paint_mode = std::env::var("PAINT").is_ok();
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        cx.open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            |_, cx| {
                cx.new(|_| Proof {
                    start: Instant::now(),
                    stats: Default::default(),
                    reported: false,
                    surface,
                    lod,
                    paint_mode,
                    clicks: 0,
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
