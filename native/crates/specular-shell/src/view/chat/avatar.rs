//! The avatar beside a thread: drawn by an [`AvatarRenderer`] on the
//! compositor's device and shown as an image.
//!
//! An avatar animates only while its thread's agent works. Otherwise it
//! keeps the one still frame it was given and costs nothing: no draw, no
//! read, no frame asked for. One that is not laid out (the panel closed,
//! the list behind an open thread) is not drawn at all.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use gpui_kit::{
    AnyElement, ImageSource, IntoElement, RenderImage, Styled as _, Window, div, img, px,
};

use crate::avatar::{AvatarRenderer, Frames, Gpu, Gradient};
use crate::canvas as app_canvas;
use crate::surface::CanvasSurface;
use crate::theme;

/// An avatar's side, in points.
pub(super) const SIZE: f32 = 20.0;

/// Every avatar of the panel and what draws them.
struct Avatars {
    /// `None` until the canvas's surface exists.
    drawer: Option<Drawer>,
    each: HashMap<String, Frames>,
    /// When time started for the animation.
    start: Instant,
    /// Animates every avatar, working or not: `SPECULAR_AVATAR_LIVE=1`.
    always_live: bool,
}

thread_local! {
    static AVATARS: RefCell<Avatars> = RefCell::new(Avatars {
        drawer: None,
        each: HashMap::new(),
        start: Instant::now(),
        always_live: std::env::var_os("SPECULAR_AVATAR_LIVE").is_some(),
    });
}

/// The device and what draws on it.
struct Drawer {
    gpu: Gpu,
    renderer: Box<dyn AvatarRenderer>,
}

impl Drawer {
    /// The drawer on the compositor's device, once the canvas has one.
    fn ready(drawer: &mut Option<Self>) -> Option<&mut Self> {
        if drawer.is_none() {
            let gpu = app_canvas::with(|canvas| canvas.runtime.window().map(CanvasSurface::gpu))
                .flatten()?;
            let renderer = Box::new(Gradient::new(&gpu));
            *drawer = Some(Self { gpu, renderer });
        }
        drawer.as_mut()
    }
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
        let time = avatars.start.elapsed().as_secs_f32();
        let Some(Drawer { gpu, renderer }) = Drawer::ready(&mut avatars.drawer) else {
            return (None, false, Vec::new());
        };
        let gpu = &*gpu;
        let mut retired = Vec::new();
        let stale = (avatars.each.get(key)).is_some_and(|frames| frames.size() != pixels);
        if stale && let Some(frames) = avatars.each.remove(key) {
            retired.extend(frames.into_images());
        }
        let frames =
            (avatars.each.entry(key.to_owned())).or_insert_with(|| Frames::new(gpu, pixels));
        frames.collect(gpu);
        retired.extend(frames.take_retired());
        let still_missing = frames.image().is_none() && !frames.waiting();
        if live || still_missing {
            frames.draw(gpu, renderer.as_mut(), if live { time } else { 0.0 });
        }
        (frames.image(), live || frames.waiting(), retired)
    });
    drop_images(retired, window);
    if again {
        window.request_animation_frame();
    }
    match image {
        Some(image) => img(ImageSource::Render(image))
            .size(px(SIZE))
            .flex_shrink_0()
            .rounded_full()
            .into_any_element(),
        None => div()
            .size(px(SIZE))
            .flex_shrink_0()
            .rounded_full()
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
            .flat_map(Frames::into_images)
            .collect::<Vec<_>>()
    });
    drop_images(gone, window);
}
