//! Shared-texture pool counters, named after the Electron page host's
//! `PageHostStats` (`src/main/runtime/page-host.ts`).
//!
//! The two drop counters mean different things and must not be merged (ADR
//! 0038, "Correction"): `framesWithoutTexture` counts paints that arrived with
//! no texture, before any pool check; `framesDroppedForPoolPressure` counts
//! frames refused because the page already held the cap.

use serde::{Deserialize, Serialize};

/// Counter values summed over a run's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TextureStats {
    /// View-layer frames admitted.
    pub frames_received: u64,
    /// Popup-layer frames admitted.
    pub popup_frames: u64,
    /// Paints that carried no texture.
    pub(crate) frames_without_texture: u64,
    /// Frames refused at the pool cap.
    pub frames_dropped_for_pool_pressure: u64,
    /// Textures held right now, summed over pages.
    pub outstanding_textures: u64,
    /// Peak held by any one page, since the cap is per page.
    pub max_outstanding_textures: u64,
}
