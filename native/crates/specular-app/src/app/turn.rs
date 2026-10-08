//! One turn of the winit loop: let the runtime take in what happened, step
//! the benchmark, then ask for a frame if one is owed, or sleep.

use std::time::Instant;

use winit::event_loop::{ActiveEventLoop, ControlFlow};

use super::Shell;

impl Shell {
    pub(super) fn turn(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.runtime.turn();
        #[cfg(target_os = "macos")]
        self.run_menu();
        // A benchmark window keeps the title it opened with.
        if self.bench.is_none() {
            self.runtime.refresh_title();
        }
        let next_step = (self.bench.as_mut()).and_then(|bench| bench.step(&mut self.runtime, now));
        if self.runtime.closing {
            // Shutting the source down takes turns of the loop.
            event_loop.set_control_flow(ControlFlow::Poll);
            return;
        }
        if self.runtime.frame_wanted() {
            // Not `Wait`: a loop that sleeps between a present and the next
            // frame's redraw is sometimes woken a refresh late. The redraw
            // blocks on the display, so polling here does not spin.
            event_loop.set_control_flow(ControlFlow::Poll);
            if let Some(gpu) = self.runtime.gpu.as_ref() {
                gpu.window.request_redraw();
            }
            return;
        }
        let mut wake = self.runtime.next_turn();
        if let Some(step) = next_step {
            wake = wake.min(step);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(wake));
    }
}
