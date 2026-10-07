//! Image textures the scene refers to by [`ImageId`].

use specular_core::{PixelFormat, PixelSize};
use specular_scene::ImageId;

use crate::Compositor;
use crate::error::FrameImportError;
use crate::upload;

/// An uploaded image, ready to sample.
#[derive(Debug)]
pub(crate) struct ImageTexture {
    pub(crate) bind_group: wgpu::BindGroup,
}

impl Compositor {
    /// Uploads the pixels an [`ImageDraw`](specular_scene::ImageDraw) with
    /// this id shows, replacing any earlier upload. `rgba` is tightly packed
    /// sRGB with straight alpha, `size.width * 4` bytes per row.
    pub fn set_image(
        &mut self,
        id: ImageId,
        size: PixelSize,
        rgba: &[u8],
    ) -> Result<(), FrameImportError> {
        upload::validate_frame_size(size, self.device.limits().max_texture_dimension_2d)?;
        let needed = size.area() * 4;
        if (rgba.len() as u64) < needed {
            return Err(FrameImportError::ShortBuffer {
                actual: rgba.len() as u64,
                needed,
            });
        }
        let srgb = self.target_format.is_srgb();
        let texels = premultiply(&rgba[..needed as usize], srgb);
        let format = upload::page_texture_format(PixelFormat::Rgba8Unorm, srgb);
        let texture = upload::create_page_texture(&self.device, size, format);
        self.queue.write_texture(
            texture.as_image_copy(),
            &texels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.width * 4),
                rows_per_image: None,
            },
            upload::extent(size),
        );
        let bind_group = self.pipelines.texture_bind_group(&self.device, &texture);
        self.scene_pass
            .images
            .insert(id, ImageTexture { bind_group });
        Ok(())
    }

    /// Forgets an uploaded image. Items that still name it draw nothing.
    pub fn remove_image(&mut self, id: ImageId) {
        self.scene_pass.images.remove(&id);
    }
}

/// Straight-alpha sRGB texels as the premultiplied texels the quad shader
/// blends. When the texture is sampled as sRGB the multiply has to happen in
/// linear light, so the colour is decoded, scaled and encoded again.
fn premultiply(rgba: &[u8], linear_light: bool) -> Vec<u8> {
    let scale = |channel: u8, alpha: f32| {
        if linear_light {
            encode(decode(channel) * alpha)
        } else {
            (f32::from(channel) * alpha).round() as u8
        }
    };
    let mut out = Vec::with_capacity(rgba.len());
    for texel in rgba.as_chunks::<4>().0 {
        let alpha = f32::from(texel[3]) / 255.0;
        if texel[3] == 255 {
            out.extend_from_slice(texel);
        } else {
            out.extend([
                scale(texel[0], alpha),
                scale(texel[1], alpha),
                scale(texel[2], alpha),
                texel[3],
            ]);
        }
    }
    out
}

fn decode(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.040_45 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

fn encode(linear: f32) -> u8 {
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_texels_are_unchanged() {
        assert_eq!(premultiply(&[10, 200, 30, 255], true), [10, 200, 30, 255]);
    }

    #[test]
    fn gamma_space_premultiply_scales_the_bytes() {
        assert_eq!(premultiply(&[200, 100, 0, 128], false), [100, 50, 0, 128]);
    }

    #[test]
    fn linear_light_premultiply_keeps_more_of_the_encoded_value() {
        // Half of white in linear light is sRGB 188, not 128.
        assert_eq!(
            premultiply(&[255, 255, 255, 128], true),
            [188, 188, 188, 128]
        );
    }

    #[test]
    fn transparent_texels_carry_no_colour() {
        assert_eq!(premultiply(&[255, 255, 255, 0], true), [0, 0, 0, 0]);
    }
}
