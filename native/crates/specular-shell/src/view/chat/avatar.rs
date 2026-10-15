//! The avatar beside a thread: the Prism, drawn on the compositor's device
//! and shown as an image.
//!
//! An avatar animates only while its thread's agent works. Otherwise it
//! holds the picture it last showed and costs nothing: no draw, no read, no
//! frame asked for. One that is not laid out (the panel closed,
//! the list behind an open thread) is not drawn at all.
//!
//! Each avatar is its own view and wakes itself with a timer, at its
//! kind's frame rate while live. GPUI re-renders every view above one that
//! changes, and the Kit's root renders the shell's view on every frame, so
//! a live avatar still re-renders the whole window, but at its own rate
//! rather than at the display's.
//!
//! A thread shows its avatar beside its row in the list ([`Kind::Thread`]),
//! and, larger, at the end of its open transcript ([`Kind::Working`]): at
//! rest while the agent waits, tumbling at the head of the run bar while it
//! works.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::{
    App, AppContext as _, Context, Entity, ImageSource, IntoElement, Render, RenderImage,
    Styled as _, Task, Window, div, img, px,
};

use specular_prism::{Pose, Tumble};

use crate::avatar::{Frames, Gpu, Prism, STILL};
use crate::canvas as app_canvas;
use crate::surface::CanvasSurface;
use crate::theme;

/// The corner radius of the grey square shown until the first frame lands,
/// near the glass's own.
const PLACEHOLDER_RADIUS: f32 = 6.0;

/// How early a timer may wake and still find its picture due.
const EARLY: Duration = Duration::from_millis(2);

/// How soon to look again while a read is landing or the renderer builds.
const POLL: Duration = Duration::from_millis(16);

/// How far apart in time two threads' avatars can start, in seconds, so
/// that each thread shows its own colours.
const VARIETY: f32 = 30.0;

/// Where an avatar is shown, which sets its size, framing and motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Kind {
    /// Beside a thread's row in the list.
    Thread,
    /// At the end of the open transcript, tumbling while the agent works.
    Working,
}

impl Kind {
    /// The side, in points.
    fn size(self) -> f32 {
        match self {
            Self::Thread => 20.0,
            Self::Working => 40.0,
        }
    }

    /// Half the frame's edge in world units. The glass's half-edge is 0.39:
    /// a still avatar is framed to it with a hair for the rim, and a
    /// tumbling one with room for its corners to swing out (25 points of
    /// cube at rest in 40).
    fn half_extent(self) -> f32 {
        match self {
            Self::Thread => 0.40,
            Self::Working => 0.62,
        }
    }

    /// The least time between two live pictures. Each costs about a
    /// millisecond of the main thread in a debug build: the slow-moving
    /// thread avatar draws at 30 fps, and the tumble, which steps at 30,
    /// at 60.
    fn frame(self) -> Duration {
        match self {
            Self::Thread => Duration::from_millis(33),
            Self::Working => Duration::from_micros(16_667),
        }
    }
}

/// The panel's avatars, one view per thread and kind.
pub(super) struct Avatars {
    drawer: Rc<RefCell<Drawer>>,
    each: HashMap<(Kind, String), Entity<ThreadAvatar>>,
}

impl Avatars {
    pub(super) fn new() -> Self {
        Self {
            drawer: Rc::new(RefCell::new(Drawer {
                gpu: None,
                prisms: HashMap::new(),
                always_live: std::env::var_os("SPECULAR_AVATAR_LIVE").is_some(),
            })),
            each: HashMap::new(),
        }
    }

    /// The avatar of the thread `key`, animated while `working`.
    pub(super) fn of(&mut self, key: &str, working: bool, cx: &mut App) -> Entity<ThreadAvatar> {
        self.get(Kind::Thread, key, working, cx)
    }

    /// The transcript's avatar of the thread `key`, tumbling while `working`.
    pub(super) fn working(
        &mut self,
        key: &str,
        working: bool,
        cx: &mut App,
    ) -> Entity<ThreadAvatar> {
        self.get(Kind::Working, key, working, cx)
    }

    /// Whether `SPECULAR_AVATAR_LIVE` is set: every avatar animates as if
    /// its agent worked, and the open thread shows a run bar.
    pub(super) fn always_live(&self) -> bool {
        self.drawer.borrow().always_live
    }

    fn get(&mut self, kind: Kind, key: &str, working: bool, cx: &mut App) -> Entity<ThreadAvatar> {
        let avatar = (self.each.entry((kind, key.to_owned()))).or_insert_with(|| {
            let drawer = Rc::clone(&self.drawer);
            cx.new(|_| ThreadAvatar::new(kind, key, drawer))
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
        let leaving: Vec<(Kind, String)> = (self.each.keys())
            .filter(|(_, key)| !keep.contains(&key.as_str()))
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
    /// A renderer per kind, at the size that kind is drawn at.
    prisms: HashMap<Kind, Prism>,
    /// Animates every avatar, working or not: `SPECULAR_AVATAR_LIVE=1`, a
    /// debug knob for a scripted run, which cannot start an agent.
    always_live: bool,
}

impl Drawer {
    /// The device and `kind`'s renderer `pixels` square, once the canvas
    /// has a surface.
    fn parts(&mut self, kind: Kind, pixels: u32) -> Option<(&Gpu, &mut Prism)> {
        if self.gpu.is_none() {
            self.gpu = app_canvas::with(|canvas| canvas.runtime.window().map(CanvasSurface::gpu))
                .flatten();
        }
        let gpu = self.gpu.as_ref()?;
        let prism = (self.prisms.entry(kind))
            .or_insert_with(|| Prism::build(gpu, pixels, kind.half_extent()));
        if prism.size() != pixels {
            *prism = Prism::build(gpu, pixels, kind.half_extent());
        }
        Some((gpu, prism))
    }
}

/// One thread's avatar.
pub(super) struct ThreadAvatar {
    kind: Kind,
    drawer: Rc<RefCell<Drawer>>,
    working: bool,
    /// How a working avatar is turned; `None` for one that holds still.
    tumble: Option<Tumble>,
    /// `None` until the canvas's surface exists.
    frames: Option<Frames>,
    clock: Clock,
    /// When its next live picture is due.
    due: Option<Instant>,
    /// The timer that renders it again.
    wake: Option<Task<()>>,
}

impl ThreadAvatar {
    fn new(kind: Kind, key: &str, drawer: Rc<RefCell<Drawer>>) -> Self {
        Self {
            kind,
            drawer,
            working: false,
            tumble: (kind == Kind::Working).then(|| Tumble::new(seed(key))),
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
        let working = self.working || drawer.always_live;
        // A tumble still settling keeps it moving after the work stops.
        let live = working || self.tumble.as_ref().is_some_and(|tumble| !tumble.at_rest());
        let Some((gpu, prism)) = drawer.parts(self.kind, pixels) else {
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
        let (time, elapsed) = self.clock.tick(live);
        if let Some(tumble) = &mut self.tumble {
            tumble.advance(elapsed, working);
        }
        let pose = (self.tumble.as_ref()).map_or(Pose::IDENTITY, Tumble::pose);
        let frame = self.kind.frame();
        let now = Instant::now();
        let due = self.due.is_none_or(|due| now + EARLY >= due);
        if prism.ready() && ((live && due) || still_missing) {
            frames.draw(gpu, prism, time, pose);
            // A frame that lands late keeps the cadence; one after a pause
            // starts it again.
            self.due = Some(match self.due {
                Some(due) if now < due + frame => due + frame,
                _ => now + frame,
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
        let size = self.kind.size();
        let pixels = (size * window.scale_factor()).round().max(1.0) as u32;
        let (image, next, retired) = self.step(pixels);
        drop_images(retired, window);
        if let Some(after) = next {
            self.wake_after(after, cx);
        }
        match image {
            Some(image) => img(ImageSource::Render(image))
                .size(px(size))
                .flex_shrink_0()
                .into_any_element(),
            None => div()
                .size(px(size))
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
            start: STILL + (seed(key) % 1000) as f32 / 1000.0 * VARIETY,
            run: 0.0,
            last: None,
        }
    }

    /// The scene time to draw and the time since the last tick, the clock
    /// moving only while `live`.
    fn tick(&mut self, live: bool) -> (f32, Duration) {
        let mut elapsed = Duration::ZERO;
        if live {
            let now = Instant::now();
            if let Some(last) = self.last {
                elapsed = now.duration_since(last);
                self.run += elapsed.as_secs_f32();
            }
            self.last = Some(now);
        } else {
            self.last = None;
        }
        (self.start + self.run, elapsed)
    }
}

/// A number that a thread's id always gives (FNV-1a).
fn seed(key: &str) -> u64 {
    let hash = (key.bytes()).fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    u64::from(hash)
}

/// Lets GPUI's atlas go of images no longer shown.
fn drop_images(images: Images, window: &mut Window) {
    for image in images {
        if let Err(error) = window.drop_image(image) {
            tracing::warn!("dropping an avatar frame: {error}");
        }
    }
}
