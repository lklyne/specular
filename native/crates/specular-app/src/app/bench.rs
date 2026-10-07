//! The benchmark side of the shell: replaying gesture profiles on the
//! camera, and what a run reports when it ends.

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_bench::{BenchLine, GestureProfile, InputLatencyLine};
use specular_doc::ItemId;
use specular_interact::{Action, Event};

use super::{START_CAMERA, Shell};
use crate::bench_run::{BenchRun, BenchTick, RunSource};

impl Shell {
    /// Starts replaying `profiles` once the warmup has passed.
    pub(super) fn start_bench(&mut self, profiles: Vec<GestureProfile>, step_interval: Duration) {
        if self.options.chrome {
            // The selection outline and handles belong in every measured frame.
            let first = self.app.pages().next().map(|(id, ..)| id.clone());
            let selection = first.into_iter().map(ItemId::Entity).collect();
            self.dispatch(Event::Action(Action::Select(selection)));
        }
        self.bench = Some(BenchRun::new(
            profiles,
            self.options.warmup,
            step_interval,
            START_CAMERA,
            RunSource {
                name: self.source.name(),
                representative: self.options.representative_source,
                pages: self.hosts.len(),
                paint_policy: self.options.paint_policy,
                chrome: self.options.chrome,
                annotations: self.options.annotations,
            },
            Instant::now(),
        ));
    }

    /// Moves the camera one step along the running profile. Returns whether
    /// the run has just finished, in which case the shell is shutting down.
    pub(super) fn tick_bench(&mut self, viewport: Vec2) -> anyhow::Result<bool> {
        let Some(bench) = self.bench.as_mut() else {
            return Ok(false);
        };
        let mut camera = self.app.session().camera;
        if bench.tick(Instant::now(), &mut camera, viewport / 2.0) == BenchTick::Finished {
            self.finish_bench()?;
            return Ok(true);
        }
        if camera != self.app.session().camera {
            self.dispatch(Event::Action(Action::SetCamera(camera)));
        }
        Ok(false)
    }

    fn finish_bench(&mut self) -> anyhow::Result<()> {
        if let Some(bench) = self.bench.take() {
            for report in bench.reports() {
                println!("{}", serde_json::to_string(report)?);
            }
        }
        self.exit();
        Ok(())
    }

    /// Reports the session's input latency (a JSON line for `assemble`) and
    /// import-cache use.
    pub(super) fn report_session(&self) {
        let latency = self.latency.summary();
        if latency.samples > 0 {
            tracing::info!(
                samples = latency.samples,
                unresolved = latency.unresolved,
                p50_ms = latency.p50_ms,
                p95_ms = latency.p95_ms,
                max_ms = latency.max_ms,
                "input to present"
            );
            let line = BenchLine::InputLatency(InputLatencyLine {
                input_latency: latency,
            });
            match serde_json::to_string(&line) {
                Ok(json) => println!("{json}"),
                Err(error) => tracing::warn!("cannot report input latency: {error}"),
            }
        }
        if let Some(gpu) = self.gpu.as_ref() {
            let (hits, misses) = gpu.compositor.import_cache_hits_and_misses();
            if hits + misses > 0 {
                tracing::info!(hits, misses, "shared-surface import cache");
            }
        }
    }
}
