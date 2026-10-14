//! One avatar's frames: the buffers its pictures are read back through,
//! and the image GPUI shows.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use gpui_kit::RenderImage;
use image::{Frame, RgbaImage};
use specular_prism::Pose;

use super::{Gpu, Prism};

/// How many reads may be in flight: one landing while the next is drawn.
const SLOTS: usize = 2;

const FREE: u8 = 0;
const MAPPING: u8 = 1;
const MAPPED: u8 = 2;

/// A readback buffer and where its read is.
#[derive(Debug)]
struct Slot {
    buffer: wgpu::Buffer,
    state: Arc<AtomicU8>,
    /// Which draw it holds, so the newest of two landed reads is shown.
    draw: u64,
}

/// One avatar at one size.
#[derive(Debug)]
pub(crate) struct Frames {
    size: u32,
    /// Bytes from one row to the next in a readback buffer.
    stride: u32,
    slots: [Slot; SLOTS],
    draws: u64,
    shown: Option<(u64, Arc<RenderImage>)>,
    /// Images that were shown and are to leave GPUI's atlas.
    retired: Vec<Arc<RenderImage>>,
}

impl Frames {
    /// An avatar `size` device pixels square.
    pub(crate) fn new(gpu: &Gpu, size: u32) -> Self {
        let size = size.max(1);
        let stride = (size * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let slot = || Slot {
            buffer: gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("avatar-readback"),
                size: u64::from(stride) * u64::from(size),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            state: Arc::new(AtomicU8::new(FREE)),
            draw: 0,
        };
        Self {
            size,
            stride,
            slots: [slot(), slot()],
            draws: 0,
            shown: None,
            retired: Vec::new(),
        }
    }

    /// The size in device pixels.
    pub(crate) fn size(&self) -> u32 {
        self.size
    }

    /// Draws the picture at `time` in `pose` and starts reading it back. Does nothing
    /// while every buffer is still being read (the GPU is behind) or while
    /// `prism` is not ready. `prism` draws at this avatar's size.
    pub(crate) fn draw(&mut self, gpu: &Gpu, prism: &mut Prism, time: f32, pose: Pose) {
        debug_assert_eq!(prism.size(), self.size);
        let Some(slot) =
            (self.slots.iter_mut()).find(|slot| slot.state.load(Ordering::Acquire) == FREE)
        else {
            return;
        };
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("avatar"),
            });
        let Some(texture) = prism.render(&mut encoder, &gpu.queue, time, pose) else {
            return;
        };
        self.draws += 1;
        slot.draw = self.draws;
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &slot.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.stride),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        gpu.queue.submit([encoder.finish()]);
        slot.state.store(MAPPING, Ordering::Release);
        let state = Arc::clone(&slot.state);
        slot.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                state.store(
                    if result.is_ok() { MAPPED } else { FREE },
                    Ordering::Release,
                );
            });
    }

    /// Takes the reads that have landed and shows the newest. Returns
    /// whether the image changed.
    pub(crate) fn collect(&mut self, gpu: &Gpu) -> bool {
        if (self.slots.iter()).all(|slot| slot.state.load(Ordering::Acquire) != MAPPING) {
            // Nothing is on its way.
        } else if let Err(error) = gpu.device.poll(wgpu::PollType::Poll) {
            tracing::warn!("avatar poll failed: {error}");
        }
        let mut changed = false;
        let mut landed: Vec<usize> = (0..SLOTS)
            .filter(|&index| self.slots[index].state.load(Ordering::Acquire) == MAPPED)
            .collect();
        landed.sort_by_key(|&index| self.slots[index].draw);
        for index in landed {
            let slot = &self.slots[index];
            let newer = self
                .shown
                .as_ref()
                .is_none_or(|(draw, _)| slot.draw > *draw);
            if newer && let Some(image) = self.read(slot) {
                let old = self.shown.replace((slot.draw, image));
                self.retired.extend(old.map(|(_, image)| image));
                changed = true;
            }
            slot.buffer.unmap();
            slot.state.store(FREE, Ordering::Release);
        }
        changed
    }

    /// The landed pixels of `slot`, rows unpadded, as a GPUI image.
    fn read(&self, slot: &Slot) -> Option<Arc<RenderImage>> {
        let row = (self.size * 4) as usize;
        let bytes: Vec<u8> = {
            let data = slot.buffer.slice(..).get_mapped_range().ok()?;
            (data.chunks_exact(self.stride as usize))
                .flat_map(|line| &line[..row])
                .copied()
                .collect()
        };
        // GPUI reads the bytes as BGRA, which is what the prism wrote.
        let buffer = RgbaImage::from_raw(self.size, self.size, bytes)?;
        Some(Arc::new(RenderImage::new(vec![Frame::new(buffer)])))
    }

    /// The image to show, once one has been read back.
    pub(crate) fn image(&self) -> Option<Arc<RenderImage>> {
        self.shown.as_ref().map(|(_, image)| Arc::clone(image))
    }

    /// Whether a read is still on its way.
    pub(crate) fn waiting(&self) -> bool {
        (self.slots.iter()).any(|slot| slot.state.load(Ordering::Acquire) != FREE)
    }

    /// The images no longer shown, for the window to drop from its atlas.
    pub(crate) fn take_retired(&mut self) -> Vec<Arc<RenderImage>> {
        std::mem::take(&mut self.retired)
    }

    /// Every image this avatar holds, as it goes away.
    pub(crate) fn into_images(self) -> Vec<Arc<RenderImage>> {
        let mut images = self.retired;
        images.extend(self.shown.map(|(_, image)| image));
        images
    }
}
