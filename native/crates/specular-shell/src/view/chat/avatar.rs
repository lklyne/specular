//! The avatar beside a thread: the Prism, drawn on the compositor's device
//! and shown as an image.
//!
//! An avatar animates only while its thread's agent works. Otherwise it
//! holds the picture it last showed and costs nothing: no draw, no read, no
//! frame asked for. One that is not laid out (the panel closed,
//! the list behind an open thread) is not drawn at all.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::{
    AnyElement, ImageSource, IntoElement, RenderImage, Styled as _, Window, div, img, px,
};

use crate::avatar::{Frames, Gpu, Prism, STILL};
use crate::canvas as app_canvas;
use crate::surface::CanvasSurface;
use crate::theme;

/// An avatar's side, in points.
pub(super) const SIZE: f32 = 20.0;

/// The corner radius of the grey square shown until the first frame lands,
/// near the glass's own.
const PLACEHOLDER_RADIUS: f32 = 6.0;

/// The least time between two pictures of a live avatar. The prism moves
/// slowly, and each picture costs about a millisecond of the main thread
/// in a debug build, so a live avatar draws at 30 fps.
const LIVE_FRAME: Duration = Duration::from_millis(33);

/// How far apart in time two threads' avatars can start, in seconds, so
/// that each thread shows its own colours.
const VARIETY: f32 = 30.0;

/// Every avatar of the panel and what draws them.
struct Avatars {
    /// `None` until the canvas's surface exists.
    gpu: Option<Gpu>,
    /// The renderer at the size the avatars are drawn at.
    prism: Option<Prism>,
    each: HashMap<String, Avatar>,
    /// Animates every avatar, working or not: `SPECULAR_AVATAR_LIVE=1`, a
    /// debug knob for a scripted run, which cannot start an agent.
    always_live: bool,
}

thread_local! {
    static AVATARS: RefCell<Avatars> = RefCell::new(Avatars {
        gpu: None,
        prism: None,
        each: HashMap::new(),
        always_live: std::env::var_os("SPECULAR_AVATAR_LIVE").is_some(),
    });
}

/// One thread's avatar.
struct Avatar {
    frames: Frames,
    clock: Clock,
    /// When it was last drawn.
    drawn: Option<Instant>,
}

/// An avatar's own time, which runs only while it is live, so an avatar
/// that starts moving starts from the picture it held, and one that stops
/// holds the picture it stopped at.
struct Clock {
    /// Seconds of scene time at the still pose, different per thread.
    start: f32,
    /// Seconds it has been live.
    run: f32,
    /// When it was last drawn live.
    last: Option<Instant>,
}

impl Clock {
    fn of(key: &str) -> Self {
        Self {
            start: STILL + seed(key) * VARIETY,
            run: 0.0,
            last: None,
        }
    }

    /// The scene time to draw, the clock moving only while `live`.
    fn tick(&mut self, live: bool) -> f32 {
        if live {
            let now = Instant::now();
            if let Some(last) = self.last {
                self.run += now.duration_since(last).as_secs_f32();
            }
            self.last = Some(now);
        } else {
            self.last = None;
        }
        self.start + self.run
    }
}

/// A number in `0..1` that a thread's id always gives (FNV-1a).
fn seed(key: &str) -> f32 {
    let hash = (key.bytes()).fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    (hash % 1000) as f32 / 1000.0
}

/// Lets GPUI's atlas go of images no longer shown.
fn drop_images(images: Vec<Arc<RenderImage>>, window: &mut Window) {
    for image in images {
        if let Err(error) = window.drop_image(image) {
            tracing::warn!("dropping an avatar frame: {error}");
        }
    }
}

/// The avatar of the thread `key`, animated while `working`.
pub(super) fn thread_avatar(key: &str, working: bool, window: &mut Window) -> AnyElement {
    let pixels = (SIZE * window.scale_factor()).round().max(1.0) as u32;
    let (image, again, retired) = AVATARS.with_borrow_mut(|avatars| {
        let live = working || avatars.always_live;
        if avatars.gpu.is_none() {
            avatars.gpu =
                app_canvas::with(|canvas| canvas.runtime.window().map(CanvasSurface::gpu))
                    .flatten();
        }
        let Some(gpu) = &avatars.gpu else {
            return (None, false, Vec::new());
        };
        let prism = match &mut avatars.prism {
            Some(prism) if prism.size() == pixels => prism,
            slot => slot.insert(Prism::build(gpu, pixels)),
        };
        let mut retired = Vec::new();
        let stale = (avatars.each.get(key)).is_some_and(|avatar| avatar.frames.size() != pixels);
        if stale && let Some(avatar) = avatars.each.get_mut(key) {
            let old = std::mem::replace(&mut avatar.frames, Frames::new(gpu, pixels));
            retired.extend(old.into_images());
        }
        let avatar = (avatars.each.entry(key.to_owned())).or_insert_with(|| Avatar {
            frames: Frames::new(gpu, pixels),
            clock: Clock::of(key),
            drawn: None,
        });
        let frames = &mut avatar.frames;
        frames.collect(gpu);
        retired.extend(frames.take_retired());
        let still_missing = frames.image().is_none() && !frames.waiting();
        let time = avatar.clock.tick(live);
        let due = (avatar.drawn).is_none_or(|drawn| drawn.elapsed() >= LIVE_FRAME);
        if prism.ready() && ((live && due) || still_missing) {
            frames.draw(gpu, prism, time);
            avatar.drawn = Some(Instant::now());
        }
        let again = live || frames.waiting() || prism.building();
        (frames.image(), again, retired)
    });
    drop_images(retired, window);
    if again {
        window.request_animation_frame();
    }
    match image {
        Some(image) => img(ImageSource::Render(image))
            .size(px(SIZE))
            .flex_shrink_0()
            .into_any_element(),
        None => div()
            .size(px(SIZE))
            .flex_shrink_0()
            .rounded(px(PLACEHOLDER_RADIUS))
            .bg(theme::solid(theme::zinc_100()))
            .into_any_element(),
    }
}

/// Forgets the avatars of threads no longer listed.
pub(super) fn keep_only<'a>(keys: impl IntoIterator<Item = &'a str>, window: &mut Window) {
    let keep: Vec<&str> = keys.into_iter().collect();
    let gone = AVATARS.with_borrow_mut(|avatars| {
        let leaving: Vec<String> = (avatars.each.keys())
            .filter(|key| !keep.contains(&key.as_str()))
            .cloned()
            .collect();
        (leaving.into_iter())
            .filter_map(|key| avatars.each.remove(&key))
            .flat_map(|avatar| avatar.frames.into_images())
            .collect::<Vec<_>>()
    });
    drop_images(gone, window);
}
