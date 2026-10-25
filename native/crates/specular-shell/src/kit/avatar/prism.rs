//! The Prism as an avatar: framed to fill its slot, built off the main
//! thread, one renderer for every avatar of a size.

use std::sync::mpsc::{Receiver, TryRecvError};

use specular_prism::{Alpha, Pose, PrismOptions, PrismRenderer};

use super::{FORMAT, Gpu};

/// Scene pixels per avatar pixel on each axis. 3 draws the rim and the
/// refraction's colour fringes as cleanly as 4 at 40 px; 2 softens them.
const SUPERSAMPLE: u32 = 3;
/// The shells' subdivision; at 40 px it looks the same as the web's 80.
const SHELL_SEGMENTS: u32 = 24;

/// Seconds into the scene of an avatar that has never moved: the web
/// build's key light has eased in by then, and its canonical frame is here.
pub(crate) const STILL: f32 = 4.0;

#[derive(Debug)]
enum State {
    Building(Receiver<PrismRenderer>),
    Ready(Box<PrismRenderer>),
    /// The build's thread went away without a renderer.
    Failed,
}

/// The Prism renderer for one avatar size.
#[derive(Debug)]
pub(crate) struct Prism {
    size: u32,
    state: State,
}

impl Prism {
    /// Starts building a renderer `size` device pixels square, showing
    /// `half_extent` world units either side of the centre.
    pub(crate) fn build(gpu: &Gpu, size: u32, half_extent: f32) -> Self {
        let (send, receive) = std::sync::mpsc::channel();
        let gpu = gpu.clone();
        let spawned = std::thread::Builder::new()
            .name("avatar-prism".into())
            .spawn(move || {
                let options = PrismOptions {
                    format: FORMAT,
                    alpha: Alpha::Straight,
                    shell_segments: SHELL_SEGMENTS,
                    ..PrismOptions::new(size, SUPERSAMPLE)
                };
                let mut prism = PrismRenderer::new(&gpu.device, &gpu.queue, options);
                prism.params_mut().camera.half_extent = half_extent;
                // The receiver is gone only if the avatars were.
                let _ = send.send(prism);
            });
        let state = match spawned {
            Ok(_) => State::Building(receive),
            Err(error) => {
                tracing::warn!("avatar renderer thread: {error}");
                State::Failed
            }
        };
        Self { size, state }
    }

    /// The size it draws.
    pub(crate) fn size(&self) -> u32 {
        self.size
    }

    /// Whether it can draw yet.
    pub(crate) fn ready(&mut self) -> bool {
        if let State::Building(receive) = &self.state {
            match receive.try_recv() {
                Ok(prism) => self.state = State::Ready(Box::new(prism)),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!("the avatar renderer was not built");
                    self.state = State::Failed;
                }
            }
        }
        matches!(self.state, State::Ready(_))
    }

    /// Whether it is still being built.
    pub(crate) fn building(&self) -> bool {
        matches!(self.state, State::Building(_))
    }

    /// Records the picture at `time` seconds in `pose` and returns the texture it is
    /// in, a [`FORMAT`] texture [`Self::size`] square, transparent outside
    /// the shape. The texture is shared by every avatar: copy it out before
    /// the next render, and submit between renders.
    pub(crate) fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        time: f32,
        pose: Pose,
    ) -> Option<&wgpu::Texture> {
        match &mut self.state {
            State::Ready(prism) => {
                prism.params_mut().pose = pose;
                Some(prism.render(encoder, queue, time))
            }
            State::Building(_) | State::Failed => None,
        }
    }
}
