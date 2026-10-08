//! Page paint LOD, ported from the Electron page host so both shells do the
//! same work per page: `page-frame-rate.ts` (frame-rate tiers),
//! `page-texture-scale.ts` (texture-scale tiers) and the painting and settle
//! rules in `page-host.ts` / `layout-engine.ts`.
//!
//! A page's *display scale* is screen pixels per CSS pixel. It earns a frame
//! rate (60/30/15) and a texture scale (1/0.5/0.25) by that scale, each with
//! hysteresis so a zoom hovering near a boundary does not flap. Texture
//! shrinks and grows wait for the camera to settle; an off-screen page stops
//! painting and wakes at the texture scale it is owed.

use std::time::{Duration, Instant};

/// The full paint rate (`FULL_FRAME_RATE`).
pub(crate) const FULL_FRAME_RATE: u32 = 60;

/// `(min display scale, fps)`, large to small (`TIERS`).
const FRAME_RATE_TIERS: [(f32, u32); 3] = [(0.44, FULL_FRAME_RATE), (0.25, 30), (0.0, 15)];

/// Fraction past a frame-rate boundary the scale must travel to switch.
const FRAME_RATE_HYSTERESIS: f32 = 0.1;

/// How far below a texture boundary the scale must fall before shrinking.
const TEXTURE_SHRINK_MARGIN: f32 = 0.8;

/// Settle wait before a texture grows (`TEXTURE_GROW_SETTLE_MS`).
const TEXTURE_GROW_SETTLE: Duration = Duration::from_millis(120);

/// Settle wait before a texture shrinks (`TEXTURE_SHRINK_SETTLE_MS`).
const TEXTURE_SHRINK_SETTLE: Duration = Duration::from_millis(600);

/// Texture resolution relative to full (CSS size × window scale).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextureTier {
    /// Full resolution.
    Full,
    /// Half resolution per axis.
    Half,
    /// Quarter resolution per axis.
    Quarter,
}

impl TextureTier {
    /// Large to small.
    const ALL: [Self; 3] = [Self::Full, Self::Half, Self::Quarter];

    /// Multiplier on the window scale factor.
    pub(crate) const fn factor(self) -> f32 {
        match self {
            Self::Full => 1.0,
            Self::Half => 0.5,
            Self::Quarter => 0.25,
        }
    }

    /// The smallest tier that still out-resolves `display_scale`.
    fn smallest_sharp(display_scale: f32) -> Self {
        Self::ALL
            .into_iter()
            .rev()
            .find(|tier| tier.factor() >= display_scale)
            .unwrap_or(Self::Full)
    }
}

fn tier_fps(display_scale: f32) -> u32 {
    FRAME_RATE_TIERS
        .iter()
        .find(|&&(min, _)| display_scale >= min)
        .map_or(FRAME_RATE_TIERS[2].1, |&(_, fps)| fps)
}

/// `frameRateForDisplayScale`: the rate a page at `display_scale` should
/// paint at, given the rate it paints at now.
pub(crate) fn frame_rate_for_display_scale(display_scale: f32, current: u32) -> u32 {
    let target = tier_fps(display_scale);
    if target == current {
        return current;
    }
    // The nudged scale's tier, so a jump across two boundaries that lands
    // inside the far one's margin still owes the tier in between.
    let nudged = if target > current {
        display_scale * (1.0 - FRAME_RATE_HYSTERESIS)
    } else {
        display_scale * (1.0 + FRAME_RATE_HYSTERESIS)
    };
    tier_fps(nudged)
}

/// `textureScaleForDisplayScale`: the tier a page at `display_scale` should
/// paint at, given the tier it paints at now. Growing has no margin (a thin
/// texture shows as blur); shrinking is only a saving and has one.
pub(crate) fn texture_tier_for_display_scale(
    display_scale: f32,
    current: TextureTier,
) -> TextureTier {
    let sharp = TextureTier::smallest_sharp(display_scale);
    if sharp.factor() >= current.factor() {
        return sharp;
    }
    let shrunk = TextureTier::smallest_sharp(display_scale / TEXTURE_SHRINK_MARGIN);
    if shrunk.factor() < current.factor() {
        shrunk
    } else {
        current
    }
}

/// What a [`PageLod::update`] asks the page source to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct LodChange {
    /// New paint rate.
    pub(crate) frame_rate: Option<u32>,
    /// Start or stop painting.
    pub(crate) painting: Option<bool>,
    /// New texture tier.
    pub(crate) texture: Option<TextureTier>,
}

/// One page's paint LOD state (a `PageHost`'s tier, scale and painting).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PageLod {
    frame_rate: u32,
    painting: bool,
    texture: TextureTier,
    wanted_texture: TextureTier,
    last_display_scale: Option<f32>,
    texture_due: Option<Instant>,
}

impl Default for PageLod {
    fn default() -> Self {
        Self {
            frame_rate: FULL_FRAME_RATE,
            painting: true,
            texture: TextureTier::Full,
            wanted_texture: TextureTier::Full,
            last_display_scale: None,
            texture_due: None,
        }
    }
}

impl PageLod {
    /// The state of a page created showing at `display_scale`: at the
    /// texture tier it is owed, so it never paints a full-size surface only
    /// to shrink it once the camera has settled.
    pub(crate) fn starting_at(display_scale: f32) -> Self {
        let texture = texture_tier_for_display_scale(display_scale, TextureTier::Full);
        Self {
            texture,
            wanted_texture: texture,
            ..Self::default()
        }
    }

    /// The texture tier currently applied.
    pub(crate) fn texture(&self) -> TextureTier {
        self.texture
    }

    /// One layout pass for a page showing at `display_scale` that is
    /// `on_screen` or not: `setDisplayScale` then `setPainting`.
    pub(crate) fn update(
        &mut self,
        display_scale: f32,
        on_screen: bool,
        now: Instant,
    ) -> LodChange {
        let mut change = LodChange::default();
        if self.texture_due.is_some_and(|due| now >= due) {
            self.texture_due = None;
            self.apply_texture(self.wanted_texture, &mut change);
        }
        // Only a camera that moved restarts the settle wait.
        if self
            .last_display_scale
            .is_none_or(|last| last.to_bits() != display_scale.to_bits())
        {
            self.last_display_scale = Some(display_scale);
            self.wanted_texture = texture_tier_for_display_scale(display_scale, self.texture);
            self.reconcile_texture(false, now, &mut change);
        }
        let next = frame_rate_for_display_scale(display_scale, self.frame_rate);
        if next != self.frame_rate {
            self.frame_rate = next;
            if self.painting {
                change.frame_rate = Some(next);
            }
        }
        if on_screen != self.painting {
            self.painting = on_screen;
            change.painting = Some(on_screen);
            if on_screen {
                // Nobody saw the page at its old scale, so it wakes at the one
                // it is owed, and at its current tier's rate.
                self.reconcile_texture(true, now, &mut change);
                change.frame_rate = Some(self.frame_rate);
            }
        }
        change
    }

    fn reconcile_texture(&mut self, at_once: bool, now: Instant, change: &mut LodChange) {
        self.texture_due = None;
        let target = self.wanted_texture;
        if target == self.texture {
            return;
        }
        if at_once || self.last_display_scale.is_some_and(|scale| scale >= 1.0) {
            self.apply_texture(target, change);
            return;
        }
        let settle = if target.factor() > self.texture.factor() {
            TEXTURE_GROW_SETTLE
        } else {
            TEXTURE_SHRINK_SETTLE
        };
        self.texture_due = Some(now + settle);
    }

    fn apply_texture(&mut self, tier: TextureTier, change: &mut LodChange) {
        if tier != self.texture {
            self.texture = tier;
            change.texture = Some(tier);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_rate_tiers_have_a_hysteresis_margin() {
        for (scale, current, expected) in [
            (0.5, 30, FULL_FRAME_RATE),
            (0.25, FULL_FRAME_RATE, 30),
            (0.1, 30, 15),
            // Boundary 0.44; the down-switch waits until 0.4.
            (0.42, FULL_FRAME_RATE, FULL_FRAME_RATE),
            // The up-switch waits until ~0.49.
            (0.46, 30, 30),
        ] {
            assert_eq!(
                frame_rate_for_display_scale(scale, current),
                expected,
                "{scale} from {current}"
            );
        }
    }

    #[test]
    fn texture_grows_without_margin_and_shrinks_past_one() {
        for (scale, current, expected) in [
            (0.3, TextureTier::Quarter, TextureTier::Half),
            // 0.45 is under the 0.5 boundary but not by the 0.8 margin.
            (0.45, TextureTier::Full, TextureTier::Full),
            (0.2, TextureTier::Full, TextureTier::Quarter),
        ] {
            assert_eq!(
                texture_tier_for_display_scale(scale, current),
                expected,
                "{scale} from {current:?}"
            );
        }
    }

    #[test]
    fn a_page_created_zoomed_out_starts_at_the_texture_it_is_owed() {
        let start = Instant::now();
        let mut lod = PageLod::starting_at(0.25);
        assert_eq!(lod.texture(), TextureTier::Half);
        // And is not asked to change it, at once or after the settle wait.
        let first = lod.update(0.25, true, start);
        let settled = lod.update(0.25, true, start + TEXTURE_SHRINK_SETTLE);
        assert_eq!((first.texture, settled.texture), (None, None));
        assert_eq!(PageLod::starting_at(1.0).texture(), TextureTier::Full);
    }

    #[test]
    fn texture_shrink_waits_for_the_camera_to_settle() {
        let start = Instant::now();
        let mut lod = PageLod::default();
        let first = lod.update(0.25, true, start);
        let settled = lod.update(0.25, true, start + TEXTURE_SHRINK_SETTLE);
        // 0.25 is not 0.8x under the quarter boundary, so half is owed.
        assert_eq!(
            (first.texture, settled.texture),
            (None, Some(TextureTier::Half))
        );
    }

    #[test]
    fn moving_camera_restarts_the_settle_wait() {
        let start = Instant::now();
        let mut lod = PageLod::default();
        lod.update(0.25, true, start);
        lod.update(0.24, true, start + Duration::from_millis(500));
        let change = lod.update(0.24, true, start + TEXTURE_SHRINK_SETTLE);
        assert_eq!(change.texture, None);
    }

    #[test]
    fn rate_change_while_hidden_is_deferred_to_wake() {
        let start = Instant::now();
        let mut lod = PageLod::default();
        let stopped = lod.update(1.0, false, start);
        assert_eq!(stopped.painting, Some(false));
        let hidden = lod.update(0.25, false, start);
        let woken = lod.update(0.25, true, start);
        assert_eq!((hidden.frame_rate, woken.frame_rate), (None, Some(30)));
    }

    #[test]
    fn waking_page_takes_its_texture_tier_at_once() {
        let start = Instant::now();
        let mut lod = PageLod::default();
        lod.update(1.0, false, start);
        lod.update(0.2, false, start);
        let woken = lod.update(0.2, true, start);
        assert_eq!(woken.texture, Some(TextureTier::Quarter));
    }

    #[test]
    fn full_size_page_grows_without_waiting() {
        let start = Instant::now();
        let mut lod = PageLod::default();
        lod.update(0.2, true, start);
        lod.update(0.2, true, start + TEXTURE_SHRINK_SETTLE);
        let change = lod.update(1.0, true, start + TEXTURE_SHRINK_SETTLE);
        assert_eq!(change.texture, Some(TextureTier::Full));
    }
}
