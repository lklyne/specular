//! The results file `assemble` folds a run into.
//!
//! Field names follow the Electron sources where an equivalent exists: the
//! ADR 0038 lab's per-phase result (`phase`, `durationMs`, `draws`, `drawFps`,
//! `meanFrameMs`, `maxFrameMs`, `longFrames`, `framesReceived`), the page
//! host's `PageHostStats` for texture counters, and `frameMs` for the
//! refresh interval the run stepped at.

use serde::{Deserialize, Serialize};

use crate::{BenchError, FrameSummary, LatencySummary, MemoryReport, ProfileId, TextureStats};

/// How a shell throttles page painting. Recorded in every report because the
/// policy changes how much work each page does per second, so runs under
/// different policies do not compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PaintPolicy {
    /// Electron's page-host LOD (`page-frame-rate.ts`, `page-texture-scale.ts`,
    /// `layout-engine.ts`): 60/30/15 fps by on-screen display scale with
    /// hysteresis, texture scale 1/0.5/0.25 after the camera settles, and no
    /// painting while off-screen.
    ElectronLod,
    /// Every page paints at the full rate and texture scale, on- or
    /// off-screen.
    FullRate,
}

impl PaintPolicy {
    /// The name used on command lines and in reports.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::ElectronLod => "electron-lod",
            Self::FullRate => "full-rate",
        }
    }
}

impl std::str::FromStr for PaintPolicy {
    type Err = BenchError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        [Self::ElectronLod, Self::FullRate]
            .into_iter()
            .find(|policy| policy.name() == raw)
            .ok_or_else(|| BenchError::UnknownPaintPolicy(raw.to_owned()))
    }
}

/// One run over the gesture profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    /// Where frames came from: `cef`, `synthetic`, `chromium-trace`, ...
    pub source: String,
    /// Fixture name, e.g. `static-20` or `animated-20`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture: Option<String>,
    /// Pages on the canvas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_count: Option<usize>,
    /// Refresh interval the profiles were stepped at, and the budget long
    /// frames are measured against (Electron's `frameMs`).
    pub frame_ms: f64,
    /// False when any frame came through a CPU copy or a synthetic source,
    /// which do not measure the real page cost (ADR 0038).
    pub representative: bool,
    /// The page paint policy the shell ran under (absent in older files).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paint_policy: Option<PaintPolicy>,
    /// Whether the canvas chrome layer (page borders, selection, annotations)
    /// was drawn every frame (absent in older files).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chrome: Option<bool>,
    /// Annotations the run drew every frame (absent in older files).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<usize>,
    /// Per-profile results, in run order.
    pub phases: Vec<PhaseReport>,
    /// Texture counters over the whole run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textures: Option<TextureStats>,
    /// Process-tree memory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryReport>,
    /// Forwarded input -> page repaint -> presented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_latency: Option<LatencySummary>,
    /// How the numbers were obtained, and anything that weakens them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// One profile's frame timing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseReport {
    /// Profile id.
    pub(crate) phase: ProfileId,
    /// First to last presented frame of the phase.
    pub(crate) duration_ms: f64,
    /// Presented-frame interval statistics.
    #[serde(flatten)]
    pub(crate) frames: FrameSummary,
    /// Page frames admitted during the phase (the lab's `framesReceived`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) frames_received: Option<u64>,
    /// Presented frames in which at least one visible page had no texture
    /// yet — the compositor-side gap the user sees as a blank page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) draws_without_texture: Option<u64>,
    /// Texture counters accumulated during the phase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) textures: Option<TextureStats>,
    /// The most shapes any presented frame of the phase drew (chrome load).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_shapes_drawn: Option<u64>,
}
