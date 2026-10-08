//! [`ImageMips`]: an image as the texels the GPU is given, at every mip
//! level. Building one is plain CPU work with no device, so a decode thread
//! can do it and leave the render thread only the upload.

use specular_core::PixelSize;

use crate::error::FrameImportError;
use crate::upload;

/// What a compositor needs an image prepared for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageSpec {
    /// Whether the texture is sampled as sRGB, so alpha and the mip filter
    /// have to work in linear light.
    pub linear_light: bool,
    /// The largest texture side the device takes.
    pub max_dimension: u32,
}

/// An image's premultiplied texels, full size first and each level half the
/// one before, down to one texel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMips {
    size: PixelSize,
    levels: Vec<Vec<u8>>,
}

impl ImageMips {
    /// Prepares `rgba`: tightly packed sRGB with straight alpha,
    /// `size.width * 4` bytes per row.
    pub fn build(size: PixelSize, rgba: &[u8], spec: ImageSpec) -> Result<Self, FrameImportError> {
        upload::validate_frame_size(size, spec.max_dimension)?;
        let needed = size.area() * 4;
        if (rgba.len() as u64) < needed {
            return Err(FrameImportError::ShortBuffer {
                actual: rgba.len() as u64,
                needed,
            });
        }
        let mut levels = vec![premultiply(&rgba[..needed as usize], spec.linear_light)];
        let mut level_size = size;
        while level_size.width > 1 || level_size.height > 1 {
            let (half, texels) = halve(&levels[levels.len() - 1], level_size, spec.linear_light);
            levels.push(texels);
            level_size = half;
        }
        Ok(Self { size, levels })
    }

    /// The full-size level's size.
    pub fn size(&self) -> PixelSize {
        self.size
    }

    /// Each level's size and texels, full size first.
    pub(crate) fn levels(&self) -> impl Iterator<Item = (PixelSize, &[u8])> {
        let mut size = self.size;
        self.levels.iter().map(move |texels| {
            let level = size;
            size = half_size(size);
            (level, texels.as_slice())
        })
    }

    /// How many levels there are.
    pub(crate) fn level_count(&self) -> u32 {
        self.levels.len() as u32
    }
}

fn half_size(size: PixelSize) -> PixelSize {
    PixelSize::new((size.width / 2).max(1), (size.height / 2).max(1))
}

/// The next level down: each texel the mean of the two-by-two block it
/// covers. Premultiplied texels average without colour bleeding in from
/// transparent neighbours.
fn halve(texels: &[u8], size: PixelSize, linear_light: bool) -> (PixelSize, Vec<u8>) {
    let half = half_size(size);
    let (width, height) = (size.width as usize, size.height as usize);
    let decode = decode_table();
    let mut out = Vec::with_capacity(half.width as usize * half.height as usize * 4);
    for y in 0..half.height as usize {
        let rows = [y * 2, (y * 2 + 1).min(height - 1)];
        for x in 0..half.width as usize {
            let columns = [x * 2, (x * 2 + 1).min(width - 1)];
            let mut sum = [0.0_f32; 4];
            for row in rows {
                for column in columns {
                    let at = (row * width + column) * 4;
                    for channel in 0..3 {
                        let value = texels[at + channel];
                        sum[channel] += if linear_light {
                            decode[usize::from(value)]
                        } else {
                            f32::from(value)
                        };
                    }
                    sum[3] += f32::from(texels[at + 3]);
                }
            }
            for value in &sum[..3] {
                out.push(if linear_light {
                    encode(value / 4.0)
                } else {
                    (value / 4.0).round() as u8
                });
            }
            out.push((sum[3] / 4.0).round() as u8);
        }
    }
    (half, out)
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

fn decode_table() -> [f32; 256] {
    let mut table = [0.0; 256];
    for (value, slot) in table.iter_mut().enumerate() {
        *slot = decode(value as u8);
    }
    table
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

    const GAMMA: ImageSpec = ImageSpec {
        linear_light: false,
        max_dimension: 8192,
    };
    const LINEAR: ImageSpec = ImageSpec {
        linear_light: true,
        max_dimension: 8192,
    };

    fn sizes(mips: &ImageMips) -> Vec<(u32, u32)> {
        mips.levels()
            .map(|(size, texels)| {
                assert_eq!(texels.len() as u64, size.area() * 4);
                (size.width, size.height)
            })
            .collect()
    }

    #[test]
    fn opaque_texels_are_unchanged() {
        assert_eq!(premultiply(&[10, 200, 30, 255], true), [10, 200, 30, 255]);
    }

    #[test]
    fn premultiplying_scales_colour_by_alpha_in_the_space_the_texture_is_sampled_in() {
        for (name, texel, linear_light, expected) in [
            // Half of white in linear light is sRGB 188, not 128.
            (
                "linear half",
                [255, 255, 255, 128],
                true,
                [188, 188, 188, 128],
            ),
            ("gamma half", [200, 100, 0, 128], false, [100, 50, 0, 128]),
            ("transparent", [255, 255, 255, 0], true, [0, 0, 0, 0]),
        ] {
            assert_eq!(premultiply(&texel, linear_light), expected, "{name}");
        }
    }

    #[test]
    fn levels_halve_down_to_one_texel() {
        let mips = ImageMips::build(PixelSize::new(8, 2), &[255; 8 * 2 * 4], GAMMA).unwrap();
        assert_eq!(sizes(&mips), [(8, 2), (4, 1), (2, 1), (1, 1)]);
        assert_eq!(mips.level_count(), 4);
        // An odd side rounds down and never reaches zero.
        let odd = ImageMips::build(PixelSize::new(5, 3), &[255; 5 * 3 * 4], GAMMA).unwrap();
        assert_eq!(sizes(&odd), [(5, 3), (2, 1), (1, 1)]);
    }

    #[test]
    fn a_level_is_the_mean_of_the_block_above_it() {
        // One white texel and three black.
        let texels = [[255; 4], [0, 0, 0, 255], [0, 0, 0, 255], [0, 0, 0, 255]].concat();
        let size = PixelSize::new(2, 2);
        let gamma = ImageMips::build(size, &texels, GAMMA).unwrap();
        let linear = ImageMips::build(size, &texels, LINEAR).unwrap();
        assert_eq!(gamma.levels().nth(1).unwrap().1, [64, 64, 64, 255]);
        // A quarter of white in linear light is sRGB 137.
        assert_eq!(linear.levels().nth(1).unwrap().1, [137, 137, 137, 255]);
    }

    #[test]
    fn a_transparent_neighbour_dims_a_level_without_tinting_it() {
        // Opaque red beside transparent green: the mean is half-covered red.
        let texels = [[255, 0, 0, 255], [0, 255, 0, 0]].concat();
        let mips = ImageMips::build(PixelSize::new(2, 1), &texels, GAMMA).unwrap();
        assert_eq!(mips.levels().nth(1).unwrap().1, [128, 0, 0, 128]);
    }

    #[test]
    fn a_short_buffer_and_an_oversized_image_are_refused() {
        assert_eq!(
            ImageMips::build(PixelSize::new(2, 2), &[0; 15], GAMMA),
            Err(FrameImportError::ShortBuffer {
                actual: 15,
                needed: 16
            })
        );
        let small = ImageSpec {
            max_dimension: 4,
            ..GAMMA
        };
        assert!(ImageMips::build(PixelSize::new(8, 1), &[0; 32], small).is_err());
    }
}
