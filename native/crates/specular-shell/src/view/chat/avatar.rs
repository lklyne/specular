//! The avatar beside a thread: the Prism, drawn on the compositor's device
//! and shown as an image.
//!
//! An avatar animates only while its thread's agent works. Otherwise it
//! holds the picture it last showed and costs nothing: no draw, no read, no
//! frame asked for. One that is not laid out (the panel closed,
//! the list behind an open thread) is not drawn at all.
//!
//! Each avatar is its own view and wakes itself with a timer, at
//! [`LIVE_FRAME`] while live. GPUI re-renders every view above one that
//! changes, and the Kit's root renders the shell's view on every frame, so
//! a live avatar still re-renders the whole window, but at its own 30 fps
//! rather than at the display's rate.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::{
    App, AppContext as _, Context, Entity, ImageSource, IntoElement, Render, RenderImage,
    Styled as _, Task, Window, div, img, px,
};

use crate::avatar::{Frames, Gpu, Prism, STILL};
use crate::canvas as app_canvas;
use crate::surface::CanvasSurface;
use crate::theme;

/// An avatar's side, in points.
const SIZE: f32 = 20.0;

/// The corner radius of the grey square shown until the first frame lands,
/// near the glass's own.
const PLACEHOLDER_RADIUS: f32 = 6.0;

/// The least time between two pictures of a live avatar. The prism moves
/// slowly, and each picture costs about a millisecond of the main thread
/// in a debug build, so a live avatar draws at 30 fps.
const LIVE_FRAME: Duration = Duration::from_millis(33);

/// How early a timer may wake and still find its picture due.
const EARLY: Duration = Duration::from_millis(2);

/// How soon to look again while a read is landing or the renderer builds.
const POLL: Duration = Duration::from_millis(16);

/// How far apart in time two threads' avatars can start, in seconds, so
/// that each thread shows its own colours.
const VARIETY: f32 = 30.0;

/// The panel's avatars, one view per thread.
pub(super) struct Avatars {
    drawer: Rc<RefCell<Drawer>>,
    each: HashMap<String, Entity<ThreadAvatar>>,
}

impl Avatars {
    pub(super) fn new() -> Self {
        Self {
            drawer: Rc::new(RefCell::new(Drawer {
                gpu: None,
                prism: None,
                always_live: std::env::var_os("SPECULAR_AVATAR_LIVE").is_some(),
            })),
            each: HashMap::new(),
        }
    }

    /// The avatar of the thread `key`, animated while `working`.
    pub(super) fn of(&mut self, key: &str, working: bool, cx: &mut App) -> Entity<ThreadAvatar> {
        let avatar = (self.each.entry(key.to_owned())).or_insert_with(|| {
            let drawer = Rc::clone(&self.drawer);
            cx.new(|_| ThreadAvatar::new(key, drawer))
        });
        // It renders in this frame as a child of the panel, so no notify.
        avatar.update(cx, |avatar, _| avatar.working = working);
        avatar.clone()
    }

    /// Forgets the avatars of threads no longer listed.
    pub(super) fn keep_only<'a>(
        &mut self,
        keys: impl IntoIterator<Item = &'a str>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let keep: Vec<&str> = keys.into_iter().collect();
        let leaving: Vec<String> = (self.each.keys())
            .filter(|key| !keep.contains(&key.as_str()))
            .cloned()
            .collect();
        for key in leaving {
            if let Some(avatar) = self.each.remove(&key) {
                let images = avatar.update(cx, |avatar, _| avatar.take_images());
                drop_images(images, window);
            }
        }
    }
}

/// What every avatar of the window draws with.
struct Drawer {
    /// `None` until the canvas's surface exists.
    gpu: Option<Gpu>,
    /// The renderer at the size the avatars are drawn at.
    prism: Option<Prism>,
    /// Animates every avatar, working or not: `SPECULAR_AVATAR_LIVE=1`, a
    /// debug knob for a scripted run, which cannot start an agent.
    always_live: bool,
}

impl Drawer {
    /// The device and a renderer `pixels` square, once the canvas has a
    /// surface.
    fn parts(&mut self, pixels: u32) -> Option<(&Gpu, &mut Prism)> {
        if self.gpu.is_none() {
            self.gpu = app_canvas::with(|canvas| canvas.runtime.window().map(CanvasSurface::gpu))
                .flatten();
        }
        let gpu = self.gpu.as_ref()?;
        if self
            .prism
            .as_ref()
            .is_none_or(|prism| prism.size() != pixels)
        {
            self.prism = Some(Prism::build(gpu, pixels));
        }
        Some((gpu, self.prism.as_mut()?))
    }
}

/// One thread's avatar.
pub(super) struct ThreadAvatar {
    drawer: Rc<RefCell<Drawer>>,
    working: bool,
    /// `None` until the canvas's surface exists.
    frames: Option<Frames>,
    clock: Clock,
    /// When its next live picture is due.
    due: Option<Instant>,
    /// The timer that renders it again.
    wake: Option<Task<()>>,
}

impl ThreadAvatar {
    fn new(key: &str, drawer: Rc<RefCell<Drawer>>) -> Self {
        Self {
            drawer,
            working: false,
            frames: None,
            clock: Clock::of(key),
            due: None,
            wake: None,
        }
    }

    /// Collects what landed and draws when due. Returns the image to show,
    /// how soon to render again, and the images to drop from the atlas.
    fn step(&mut self, pixels: u32) -> (Option<Arc<RenderImage>>, Option<Duration>, Images) {
        let mut drawer = self.drawer.borrow_mut();
        let live = self.working || drawer.always_live;
        let Some((gpu, prism)) = drawer.parts(pixels) else {
            return (None, Some(POLL), Vec::new());
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
        let still_missing = frames.image().is_none() && !frames.waiting();
        let time = self.clock.tick(live);
        let now = Instant::now();
        let due = self.due.is_none_or(|due| now + EARLY >= due);
        if prism.ready() && ((live && due) || still_missing) {
            frames.draw(gpu, prism, time);
            // A frame that lands late keeps the cadence; one after a pause
            // starts it again.
            self.due = Some(match self.due {
                Some(due) if now < due + LIVE_FRAME => due + LIVE_FRAME,
                _ => now + LIVE_FRAME,
            });
        }
        let next = if live {
            let wait = (self.due).map_or(POLL, |due| due.saturating_duration_since(now));
            Some(wait.max(EARLY))
        } else if frames.waiting() || prism.building() {
            Some(POLL)
        } else {
            None
        };
        (frames.image(), next, retired)
    }

    /// Renders it again in `after`, unless a timer is already set.
    fn wake_after(&mut self, after: Duration, cx: &mut Context<'_, Self>) {
        if self.wake.is_some() {
            return;
        }
        self.wake = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(after).await;
            // The avatar is gone only if its thread was.
            let _ = this.update(cx, |this, cx| {
                this.wake = None;
                cx.notify();
            });
        }));
    }

    /// Every image it holds, as it goes away.
    fn take_images(&mut self) -> Images {
        self.frames
            .take()
            .map(Frames::into_images)
            .unwrap_or_default()
    }
}

impl Render for ThreadAvatar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let pixels = (SIZE * window.scale_factor()).round().max(1.0) as u32;
        let (image, next, retired) = self.step(pixels);
        drop_images(retired, window);
        if let Some(after) = next {
            self.wake_after(after, cx);
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
}

type Images = Vec<Arc<RenderImage>>;

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
fn drop_images(images: Images, window: &mut Window) {
    for image in images {
        if let Err(error) = window.drop_image(image) {
            tracing::warn!("dropping an avatar frame: {error}");
        }
    }
}
