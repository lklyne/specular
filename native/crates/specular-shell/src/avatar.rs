//! A thread's avatar: a small animated picture that a wgpu renderer draws
//! and the Kit shows as an image.
//!
//! GPUI paints with its own Metal device and takes no wgpu texture. Its
//! `surface()` element takes only a biplanar YCbCr pixel buffer, which has
//! no alpha and half-resolution colour, so the avatar is read back instead:
//! at 24 points on a 2x display a frame is 48 x 48 x 4 bytes. A read lands a
//! frame after its draw, so the picture shown is the one finished last frame.
//!
//! The renderer runs on the compositor's device: one device, one queue, and
//! everything on the main thread, as the canvas is.

mod frames;
mod gradient;

pub(crate) use self::frames::Frames;
pub(crate) use self::gradient::Gradient;

/// The format an avatar is drawn in: what a GPUI image holds, BGRA with
/// sRGB-encoded colour and straight alpha.
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// The device an avatar is drawn on.
#[derive(Debug, Clone)]
pub(crate) struct Gpu {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}

/// Draws one frame of an avatar.
pub(crate) trait AvatarRenderer {
    /// Records the picture at `time` seconds into `target`, a [`FORMAT`]
    /// texture `size` device pixels square. Whatever is outside the avatar's
    /// shape is cleared to transparent.
    fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: u32,
        time: f32,
    );
}
