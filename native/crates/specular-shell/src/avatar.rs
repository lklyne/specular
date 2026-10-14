//! A thread's avatar: the Prism, drawn with wgpu and shown by the Kit as an
//! image.
//!
//! GPUI paints with its own Metal device and takes no wgpu texture. Its
//! `surface()` element takes only a biplanar YCbCr pixel buffer, which has
//! no alpha and half-resolution colour, so the avatar is read back instead:
//! at 20 points on a 2x display a frame is 40 x 40 x 4 bytes. A read lands a
//! frame after its draw, so the picture shown is the one finished last frame.
//!
//! The renderer runs on the compositor's device: one device, one queue, and
//! everything on the main thread, as the canvas is. Only its build, which
//! compiles the shell shaders for about half a second, runs on a thread of
//! its own.

mod frames;
mod prism;

pub(crate) use self::frames::Frames;
pub(crate) use self::prism::{Prism, STILL};

/// The format an avatar is drawn in: what a GPUI image holds, BGRA with
/// sRGB-encoded colour and straight alpha.
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// The device an avatar is drawn on.
#[derive(Debug, Clone)]
pub(crate) struct Gpu {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}
