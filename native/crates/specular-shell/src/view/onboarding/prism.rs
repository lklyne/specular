//! The Prism at the head of the first-run view, large and turning slowly.
//!
//! It turns about two axes at once at different speeds, so the tilt lags
//! the spin and the pose never quite repeats. It is drawn and read back as
//! the thread avatars are (`crate::avatar`), on the canvas's device, which
//! the window has from its start, before any space is open.

use std::f32::consts::TAU;
use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::Quat;
use gpui_kit::{
    Context, ImageSource, IntoElement, ParentElement as _, Render, RenderImage, Styled as _, Task,
    Window, div, img, px,
};
use specular_prism::Pose;

use crate::avatar::{self, Frames, Gpu, Prism, STILL};
use crate::canvas as app_canvas;
use crate::surface::CanvasSurface;

/// The side, in points.
pub(super) const SIZE: f32 = 160.0;

/// Half the frame's edge in world units: room for the cube's corners
/// (0.39 times the square root of 3) to swing out on any axis.
const HALF_EXTENT: f32 = 0.72;

/// Seconds for a whole turn about the vertical axis, and about the
/// horizontal one. Far from a ratio of small numbers, so the two drift
/// against each other.
const SPIN_PERIOD: f32 = 24.0;
const TILT_PERIOD: f32 = 37.0;

/// The least time between two pictures: 30 fps, as the thread avatars
/// draw. The turn is slow enough not to need more.
const FRAME: Duration = Duration::from_millis(33);

/// How soon to look again while a read is landing or the renderer builds.
const POLL: Duration = Duration::from_millis(16);

/// The turning Prism.
#[derive(Default)]
pub(crate) struct HeroPrism {
    /// `None` until the canvas's surface exists.
    gpu: Option<Gpu>,
    prism: Option<Prism>,
    frames: Option<Frames>,
    /// When it was first drawn: the turn's time zero.
    start: Option<Instant>,
    due: Option<Instant>,
    wake: Option<Task<()>>,
}

impl HeroPrism {
    /// Draws when due. Returns the image to show, how soon to render again
    /// (never, once its renderer failed to build) and the images to drop
    /// from the atlas.
    fn step(&mut self, pixels: u32) -> (Option<Arc<RenderImage>>, Option<Duration>, Images) {
        if self.gpu.is_none() {
            self.gpu = app_canvas::with(|canvas| canvas.runtime.window().map(CanvasSurface::gpu))
                .flatten();
        }
        let Some(gpu) = &self.gpu else {
            return (None, Some(POLL), Vec::new());
        };
        let prism = match &mut self.prism {
            Some(prism) if prism.size() == pixels => prism,
            slot => slot.insert(Prism::build(gpu, pixels, HALF_EXTENT)),
        };
        let mut retired = Vec::new();
        let frames = match &mut self.frames {
            Some(frames) if frames.size() == pixels => frames,
            slot => {
                retired.extend(slot.take().map(Frames::into_images).unwrap_or_default());
                slot.insert(Frames::new(gpu, pixels))
            }
        };
        frames.collect(gpu);
        retired.extend(frames.take_retired());
        let now = Instant::now();
        let ready = prism.ready();
        if ready && self.due.is_none_or(|due| now >= due) {
            let seconds = self.start.get_or_insert(now).elapsed().as_secs_f32();
            frames.draw(gpu, prism, STILL + seconds, turn(seconds));
            self.due = Some(now + FRAME);
        }
        let next = if ready {
            let wait = (self.due).map_or(POLL, |due| due.saturating_duration_since(now));
            Some(wait.max(Duration::from_millis(2)))
        } else if prism.building() {
            Some(POLL)
        } else {
            None
        };
        (frames.image(), next, retired)
    }

    /// Every image it holds, as it goes away.
    pub(crate) fn take_images(&mut self) -> Images {
        self.frames
            .take()
            .map(Frames::into_images)
            .unwrap_or_default()
    }
}

/// The pose `seconds` into the turn: a spin about the vertical axis, then
/// a slower tilt toward the viewer.
fn turn(seconds: f32) -> Pose {
    let spin = Quat::from_rotation_y(seconds / SPIN_PERIOD * TAU);
    let tilt = Quat::from_rotation_x(seconds / TILT_PERIOD * TAU);
    Pose {
        rotation: (tilt * spin).to_array(),
        ..Pose::IDENTITY
    }
}

type Images = Vec<Arc<RenderImage>>;

impl Render for HeroPrism {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let pixels = (SIZE * window.scale_factor()).round().max(1.0) as u32;
        let (image, next, retired) = self.step(pixels);
        avatar::drop_images(retired, window);
        if let Some(after) = next
            && self.wake.is_none()
        {
            self.wake = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(after).await;
                // It is gone only once a space opened.
                let _ = this.update(cx, |this, cx| {
                    this.wake = None;
                    cx.notify();
                });
            }));
        }
        let slot = div().size(px(SIZE)).flex_shrink_0();
        match image {
            Some(image) => slot.child(img(ImageSource::Render(image)).size_full()),
            None => slot,
        }
    }
}
