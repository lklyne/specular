//! Shared-texture pool counters, with the same names and semantics as the
//! Electron page host's `PageHostStats` (`src/main/runtime/page-host.ts`), so
//! `GET /perf/page-hosts` snapshots and Rust runs compare field for field.
//!
//! The two drop counters mean different things and must not be merged (ADR
//! 0038, "Correction"): `framesWithoutTexture` counts paints that arrived with
//! no texture, before any pool check; `framesDroppedForPoolPressure` counts
//! frames refused because the page already held the cap.

use serde::{Deserialize, Serialize};

/// Counter values for one page, or summed over pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TextureStats {
    /// View-layer frames admitted.
    pub frames_received: u64,
    /// Popup-layer frames admitted.
    pub popup_frames: u64,
    /// Paints that carried no texture.
    pub frames_without_texture: u64,
    /// Frames refused at the pool cap.
    pub frames_dropped_for_pool_pressure: u64,
    /// Textures held right now (summed over pages in a total).
    pub outstanding_textures: u64,
    /// Peak held by any one page — a per-page peak even in a total, since
    /// the cap is per page.
    pub max_outstanding_textures: u64,
}

impl TextureStats {
    /// Sums counts across pages, keeping the largest per-page peak.
    pub fn total<'a>(pages: impl IntoIterator<Item = &'a Self>) -> Self {
        pages.into_iter().fold(Self::default(), |acc, page| Self {
            frames_received: acc.frames_received + page.frames_received,
            popup_frames: acc.popup_frames + page.popup_frames,
            frames_without_texture: acc.frames_without_texture + page.frames_without_texture,
            frames_dropped_for_pool_pressure: acc.frames_dropped_for_pool_pressure
                + page.frames_dropped_for_pool_pressure,
            outstanding_textures: acc.outstanding_textures + page.outstanding_textures,
            max_outstanding_textures: acc
                .max_outstanding_textures
                .max(page.max_outstanding_textures),
        })
    }

    /// Counts accumulated between two cumulative snapshots. The peak cannot
    /// be windowed (page hosts never reset it), so `after`'s is kept and
    /// covers the page's whole life, not just the run.
    #[must_use]
    pub fn since(self, before: Self) -> Self {
        Self {
            frames_received: self.frames_received.saturating_sub(before.frames_received),
            popup_frames: self.popup_frames.saturating_sub(before.popup_frames),
            frames_without_texture: self
                .frames_without_texture
                .saturating_sub(before.frames_without_texture),
            frames_dropped_for_pool_pressure: self
                .frames_dropped_for_pool_pressure
                .saturating_sub(before.frames_dropped_for_pool_pressure),
            outstanding_textures: self.outstanding_textures,
            max_outstanding_textures: self.max_outstanding_textures,
        }
    }
}

/// The body of Electron's `GET /perf/page-hosts`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct PageHostsSnapshot {
    /// One entry per page host.
    pub hosts: Vec<TextureStats>,
}

impl PageHostsSnapshot {
    /// Counters summed over every host.
    pub fn total(&self) -> TextureStats {
        TextureStats::total(&self.hosts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_sums_counts_and_keeps_largest_peak() {
        let a = TextureStats {
            frames_received: 3,
            max_outstanding_textures: 2,
            ..TextureStats::default()
        };
        let b = TextureStats {
            frames_received: 4,
            max_outstanding_textures: 5,
            ..TextureStats::default()
        };
        let total = TextureStats::total([&a, &b]);
        assert_eq!(
            (total.frames_received, total.max_outstanding_textures),
            (7, 5)
        );
    }

    #[test]
    fn page_hosts_snapshot_parses_electron_body() {
        let body = r#"{"hosts":[
            {"pageId":"a","framesReceived":5,"popupFrames":0,"framesWithoutTexture":1,
             "framesDroppedForPoolPressure":0,"sendFailures":0,"painting":true,
             "outstandingTextures":1,"maxOutstandingTextures":3,"releaseLatencyMs":null},
            {"pageId":"b","framesReceived":2,"popupFrames":1,"framesWithoutTexture":0,
             "framesDroppedForPoolPressure":2,"sendFailures":0,
             "outstandingTextures":0,"maxOutstandingTextures":6,"releaseLatencyMs":4.2}]}"#;
        let snapshot: PageHostsSnapshot = serde_json::from_str(body).unwrap();
        assert_eq!(
            snapshot.total(),
            TextureStats {
                frames_received: 7,
                popup_frames: 1,
                frames_without_texture: 1,
                frames_dropped_for_pool_pressure: 2,
                outstanding_textures: 1,
                max_outstanding_textures: 6,
            }
        );
    }
}
