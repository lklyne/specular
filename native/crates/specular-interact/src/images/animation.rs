//! Animated images: which frame the clock is at.
//!
//! A gif's frames are all uploaded once. Each tick picks the frame for the
//! time since the animation started, and only for images in the viewport,
//! so an animation off screen costs nothing and wakes nothing.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{ImageKey, shown};
use crate::App;

/// One animated image's timing.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Animation {
    delays_ms: Arc<[u32]>,
    started_ms: u64,
    frame: u32,
}

impl Animation {
    fn total_ms(&self) -> u64 {
        self.delays_ms
            .iter()
            .map(|&delay| u64::from(delay.max(1)))
            .sum()
    }

    /// The frame shown `elapsed` milliseconds in, and how long it has left.
    fn at(&self, elapsed: u64) -> (u32, u64) {
        let mut into = elapsed % self.total_ms().max(1);
        for (frame, &delay) in self.delays_ms.iter().enumerate() {
            let delay = u64::from(delay.max(1));
            if into < delay {
                return (frame as u32, delay - into);
            }
            into -= delay;
        }
        (0, 1)
    }
}

/// Every animated image and a count of the frame changes so far.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Animations {
    by_image: BTreeMap<ImageKey, Animation>,
    epoch: u64,
}

impl Animations {
    pub(super) fn start(&mut self, key: ImageKey, delays_ms: Arc<[u32]>, now_ms: u64) {
        let animation = Animation {
            delays_ms,
            started_ms: now_ms,
            frame: 0,
        };
        self.by_image.insert(key, animation);
        self.epoch += 1;
    }

    pub(super) fn stop(&mut self, key: ImageKey) {
        if self.by_image.remove(&key).is_some() {
            self.epoch += 1;
        }
    }

    pub(super) fn frame(&self, key: ImageKey) -> u32 {
        self.by_image
            .get(&key)
            .map_or(0, |animation| animation.frame)
    }

    pub(super) fn epoch(&self) -> u64 {
        self.epoch
    }

    pub(super) fn until_next(&self, key: ImageKey, now_ms: u64) -> Option<u64> {
        let animation = self.by_image.get(&key)?;
        Some(animation.at(now_ms.saturating_sub(animation.started_ms)).1)
    }
}

/// Moves each animated image on screen to the frame the clock is at.
pub(super) fn advance(app: &mut App) {
    if app.session.images.animations.by_image.is_empty() {
        return;
    }
    let now = app.session.now_ms;
    let on_screen: Vec<ImageKey> = shown(app)
        .filter_map(|(_, file)| app.session.images.get(&file.file))
        .map(|image| image.key)
        .collect();
    let animations = &mut app.session.images.animations;
    for key in on_screen {
        let Some(animation) = animations.by_image.get_mut(&key) else {
            continue;
        };
        let (frame, _) = animation.at(now.saturating_sub(animation.started_ms));
        if frame != animation.frame {
            animation.frame = frame;
            animations.epoch += 1;
        }
    }
}
