//! One turn of the winit loop: let the runtime take in what happened, step
//! the benchmark, then ask for a frame if one is owed, or sleep.

use std::time::Instant;

use winit::event_loop::{ActiveEventLoop, ControlFlow};

use super::{GpuWindow, Shell};

impl Shell {
    pub(super) fn turn(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let now = Instant::now();
        self.runtime.turn();
        #[cfg(target_os = "macos")]
        self.run_menu();
        // A benchmark window keeps the title it opened with.
        if self.bench.is_none() {
            self.runtime.refresh_title();
        }
        let next_step = self.step_bench(now)?;
        if self.runtime.closing {
            // Shutting the source down takes turns of the loop.
            event_loop.set_control_flow(ControlFlow::Poll);
            return Ok(());
        }
        if self.runtime.frame_wanted() {
            // Not `Wait`: a loop that sleeps between a present and the next
            // frame's redraw is sometimes woken a refresh late. The redraw
            // blocks on the display, so polling here does not spin.
            event_loop.set_control_flow(ControlFlow::Poll);
            if let Some(gpu) = self.runtime.gpu.as_ref() {
                gpu.window.request_redraw();
            }
            return Ok(());
        }
        let mut wake = self.runtime.next_turn();
        if let Some(step) = next_step {
            wake = wake.min(step);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(wake));
        Ok(())
    }

    /// Steps the benchmark: once a presented frame, or once a refresh when
    /// the last step changed nothing and so drew nothing. While the run only
    /// waits (the warmup, a gap, the idle profile) it is left alone until
    /// the wait ends. Returns when to come back if no frame comes first.
    fn step_bench(&mut self, now: Instant) -> anyhow::Result<Option<Instant>> {
        let Some(bench) = self.bench.as_mut() else {
            return Ok(None);
        };
        if let Some(until) = bench.waits_until().filter(|&until| now < until) {
            if bench.keeps_presenting(now) {
                self.runtime.demand.changed();
                return Ok(Some(until));
            }
            if !self.bench_waiting {
                // One turn with nothing to draw, however long the wait.
                bench.on_skipped();
            }
            self.bench_waiting = true;
            self.bench_presented = false;
            return Ok(Some(until));
        }
        self.bench_waiting = false;
        let interval = bench.step_interval();
        let due = (self.bench_stepped_at).is_none_or(|at| now.duration_since(at) >= interval);
        if !self.bench_presented && !due {
            return Ok(self.bench_stepped_at.map(|at| at + interval));
        }
        if !self.bench_presented {
            bench.on_skipped();
        }
        self.bench_presented = false;
        self.bench_stepped_at = Some(now);
        let Some(viewport) = self.runtime.gpu.as_ref().map(GpuWindow::logical_viewport) else {
            return Ok(None);
        };
        let started = Instant::now();
        self.tick_bench(viewport)?;
        self.bench_update = started.elapsed();
        Ok(Some(now + interval))
    }
}
