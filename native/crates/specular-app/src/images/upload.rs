//! Putting what the decode thread finished on the GPU, and keeping count of
//! the textures each image holds.

use std::collections::HashMap;

use specular_compositor::{Compositor, FrameImportError, ImageMips};
use specular_interact::{ImageKey, ImageNotice};
use specular_scene::ImageId;

use super::Content;

/// How many textures each uploaded image holds: one, or one a gif frame.
#[derive(Debug, Default)]
pub(crate) struct Uploaded {
    textures: HashMap<ImageKey, u32>,
}

impl Uploaded {
    /// Uploads `content` under `key`, over what was there, and returns what
    /// to tell the app. The old textures stay until the new ones are in.
    pub(crate) fn install(
        &mut self,
        compositor: &mut Compositor,
        key: ImageKey,
        content: &Content,
    ) -> Result<ImageNotice, FrameImportError> {
        let mut put = |frame: u32, mips: &ImageMips| {
            compositor.set_image_mips(ImageId(key.texture(frame)), mips)
        };
        let (count, notice) = match content {
            Content::Still(mips) => {
                put(0, mips)?;
                let size = mips.size();
                (
                    1,
                    ImageNotice::Ready {
                        width: size.width,
                        height: size.height,
                    },
                )
            }
            Content::Vector { intrinsic, raster } => {
                put(0, raster)?;
                (
                    1,
                    ImageNotice::Ready {
                        width: intrinsic.width,
                        height: intrinsic.height,
                    },
                )
            }
            Content::Animated {
                size,
                frames,
                delays_ms,
            } => {
                for (frame, mips) in frames.iter().enumerate() {
                    put(frame as u32, mips)?;
                }
                let notice = ImageNotice::Animated {
                    width: size.width,
                    height: size.height,
                    delays_ms: delays_ms.as_slice().into(),
                };
                (frames.len() as u32, notice)
            }
        };
        let before = self.textures.insert(key, count).unwrap_or(0);
        for frame in count..before {
            compositor.remove_image(ImageId(key.texture(frame)));
        }
        Ok(notice)
    }

    /// Removes every texture of `key`.
    pub(crate) fn remove(&mut self, compositor: &mut Compositor, key: ImageKey) {
        let count = self.textures.remove(&key).unwrap_or(1);
        for frame in 0..count {
            compositor.remove_image(ImageId(key.texture(frame)));
        }
    }
}
