//! The benchmark side of the shell: replaying gesture profiles on the
//! camera, and what a run reports when it ends.

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_bench::GestureProfile;
use specular_doc::ItemId;
use specular_interact::{Action, Event};

use super::{START_CAMERA, Shell};
use crate::bench_run::{BenchRun, BenchTick, RunSource};

impl Shell {
    /// Starts replaying `profiles` once the warmup has passed.
    pub(super) fn start_bench(&mut self, profiles: Vec<GestureProfile>, step_interval: Duration) {
        if self.options.chrome {
            // The selection outline and handles belong in every measured frame.
            let first = self.runtime.app.pages().next().map(|(id, ..)| id.clone());
            let selection = first.into_iter().map(ItemId::Entity).collect();
            self.runtime
                .dispatch(Event::Action(Action::Select(selection)));
        }
        self.bench = Some(BenchRun::new(
            profiles,
            self.options.warmup,
            step_interval,
            START_CAMERA,
            RunSource {
                name: self.runtime.source.name(),
                representative: self.options.representative_source,
                pages: self.runtime.hosts.len(),
                paint_policy: self.options.paint_policy,
                chrome: self.options.chrome,
                annotations: self.options.annotations,
                canvas: (self.options.canvas.as_deref())
                    .and_then(std::path::Path::file_name)
                    .map(|name| name.to_string_lossy().into_owned()),
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
        let mut camera = self.runtime.app.session().camera;
        if bench.tick(Instant::now(), &mut camera, viewport / 2.0) == BenchTick::Finished {
            self.finish_bench()?;
            return Ok(true);
        }
        if camera != self.runtime.app.session().camera {
            // A step of a gesture, as a wheel event is.
            self.runtime.input();
            self.runtime
                .dispatch(Event::Action(Action::SetCamera(camera)));
        }
        Ok(false)
    }

    fn finish_bench(&mut self) -> anyhow::Result<()> {
        if let Some(bench) = self.bench.take() {
            for report in bench.reports() {
                println!("{}", serde_json::to_string(report)?);
            }
        }
        self.runtime.exit();
        Ok(())
    }
}
