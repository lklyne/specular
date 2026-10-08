//! Where each frame's time went and what it drew, reduced to one report a
//! profile. Presented-frame intervals say whether a frame was late; this
//! says which step made it so.

use serde::{Deserialize, Serialize};

use crate::stats::percentile;

/// One drawn frame: milliseconds in each step, and what was drawn.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameWork {
    /// `update`: applying the frame's input to the app.
    pub update_ms: f64,
    /// `view`: building the scene from the app.
    pub view_ms: f64,
    /// Resolving the scene against the camera and culling.
    pub cull_ms: f64,
    /// Shaping text that no cache held.
    pub shaping_ms: f64,
    /// Grouping items into draws.
    pub batching_ms: f64,
    /// Tessellating polygons and paths.
    pub tessellation_ms: f64,
    /// Building instances and the draw list.
    pub build_ms: f64,
    /// Laying text out as glyph quads and rasterising new glyphs.
    pub glyphs_ms: f64,
    /// Writing the frame's buffers.
    pub upload_ms: f64,
    /// Encoding the pass and submitting it.
    pub submit_ms: f64,
    /// After the submit: waiting for the GPU to finish the frame (a
    /// headless target) or for the drawable and the present (a window,
    /// where it includes the wait for vsync).
    pub gpu_ms: f64,
    /// Scene items drawn after culling.
    pub items: u32,
    /// Batches they were grouped into.
    pub batches: u32,
    /// Draw calls encoded.
    pub draw_calls: u32,
    /// Glyphs laid out.
    pub glyphs: u32,
    /// Triangles sent.
    pub triangles: u32,
}

impl FrameWork {
    /// The frame's time on the CPU: every step but the GPU wait.
    pub fn cpu_ms(&self) -> f64 {
        self.update_ms
            + self.view_ms
            + self.cull_ms
            + self.shaping_ms
            + self.batching_ms
            + self.tessellation_ms
            + self.build_ms
            + self.glyphs_ms
            + self.upload_ms
            + self.submit_ms
    }
}

/// One step over a profile's frames.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepStat {
    /// Mean milliseconds a frame.
    pub mean: f64,
    /// 95th percentile.
    pub p95: f64,
    /// The slowest frame.
    pub max: f64,
}

/// A profile's frames, step by step. Times are milliseconds a frame;
/// counts are the most any one frame drew.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkReport {
    /// Frames drawn.
    pub frames_drawn: usize,
    /// Loop turns that drew nothing because nothing had changed.
    pub frames_skipped: usize,
    /// Every step on the CPU added up, a frame.
    pub cpu: StepStat,
    /// See [`FrameWork::update_ms`].
    pub update: StepStat,
    /// See [`FrameWork::view_ms`].
    pub view: StepStat,
    /// See [`FrameWork::cull_ms`].
    pub cull: StepStat,
    /// See [`FrameWork::shaping_ms`].
    pub shaping: StepStat,
    /// See [`FrameWork::batching_ms`].
    pub batching: StepStat,
    /// See [`FrameWork::tessellation_ms`].
    pub tessellation: StepStat,
    /// See [`FrameWork::build_ms`].
    pub build: StepStat,
    /// See [`FrameWork::glyphs_ms`].
    pub glyphs: StepStat,
    /// See [`FrameWork::upload_ms`].
    pub upload: StepStat,
    /// See [`FrameWork::submit_ms`].
    pub submit: StepStat,
    /// See [`FrameWork::gpu_ms`].
    pub gpu: StepStat,
    /// Most items any frame drew.
    pub items: u32,
    /// Most batches.
    pub batches: u32,
    /// Most draw calls.
    pub draw_calls: u32,
    /// Most glyphs.
    pub glyph_count: u32,
    /// Most triangles.
    pub triangles: u32,
}

/// Collects a profile's frames.
#[derive(Debug, Clone, Default)]
pub struct WorkRecorder {
    frames: Vec<FrameWork>,
    skipped: usize,
}

impl WorkRecorder {
    /// An empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a drawn frame.
    pub fn drawn(&mut self, frame: FrameWork) {
        self.frames.push(frame);
    }

    /// Records a loop turn that drew nothing.
    pub fn skipped(&mut self) {
        self.skipped += 1;
    }

    /// The report over everything recorded.
    pub fn finish(&self) -> WorkReport {
        let stat = |step: fn(&FrameWork) -> f64| {
            let mut ms: Vec<f64> = self.frames.iter().map(step).collect();
            if ms.is_empty() {
                return StepStat::default();
            }
            ms.sort_by(f64::total_cmp);
            StepStat {
                mean: ms.iter().sum::<f64>() / ms.len() as f64,
                p95: percentile(&ms, 0.95),
                max: ms.last().copied().unwrap_or_default(),
            }
        };
        let most = |count: fn(&FrameWork) -> u32| self.frames.iter().map(count).max().unwrap_or(0);
        WorkReport {
            frames_drawn: self.frames.len(),
            frames_skipped: self.skipped,
            cpu: stat(FrameWork::cpu_ms),
            update: stat(|frame| frame.update_ms),
            view: stat(|frame| frame.view_ms),
            cull: stat(|frame| frame.cull_ms),
            shaping: stat(|frame| frame.shaping_ms),
            batching: stat(|frame| frame.batching_ms),
            tessellation: stat(|frame| frame.tessellation_ms),
            build: stat(|frame| frame.build_ms),
            glyphs: stat(|frame| frame.glyphs_ms),
            upload: stat(|frame| frame.upload_ms),
            submit: stat(|frame| frame.submit_ms),
            gpu: stat(|frame| frame.gpu_ms),
            items: most(|frame| frame.items),
            batches: most(|frame| frame.batches),
            draw_calls: most(|frame| frame.draw_calls),
            glyph_count: most(|frame| frame.glyphs),
            triangles: most(|frame| frame.triangles),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(view_ms: f64, glyphs: u32) -> FrameWork {
        FrameWork {
            view_ms,
            submit_ms: 1.0,
            gpu_ms: 5.0,
            glyphs,
            ..FrameWork::default()
        }
    }

    #[test]
    fn a_report_has_each_steps_mean_and_tail_and_the_largest_counts() {
        let mut recorder = WorkRecorder::new();
        for (view_ms, glyphs) in [(1.0, 10), (2.0, 40), (9.0, 20)] {
            recorder.drawn(frame(view_ms, glyphs));
        }
        recorder.skipped();
        let report = recorder.finish();
        assert_eq!((report.frames_drawn, report.frames_skipped), (3, 1));
        assert!((report.view.mean - 4.0).abs() < 1e-9);
        assert!((report.view.max - 9.0).abs() < 1e-9);
        assert!((report.cpu.max - 10.0).abs() < 1e-9);
        assert_eq!(report.glyph_count, 40);
    }

    #[test]
    fn a_profile_that_drew_nothing_reports_zeros() {
        let mut recorder = WorkRecorder::new();
        recorder.skipped();
        let report = recorder.finish();
        assert_eq!((report.frames_drawn, report.frames_skipped), (0, 1));
        assert_eq!(report.cpu, StepStat::default());
    }
}
