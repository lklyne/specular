//! Image textures the scene refers to by [`ImageId`].

use specular_core::{PixelFormat, PixelSize};
use specular_scene::ImageId;

use super::mips::{ImageMips, ImageSpec};
use crate::Compositor;
use crate::error::FrameImportError;
use crate::upload;

/// An uploaded image, ready to sample.
#[derive(Debug)]
pub(crate) struct ImageTexture {
    pub(crate) bind_group: wgpu::BindGroup,
}

impl Compositor {
    /// What [`ImageMips::build`] needs to prepare an image for this
    /// compositor.
    pub fn image_spec(&self) -> ImageSpec {
        ImageSpec {
            linear_light: self.target_format.is_srgb(),
            max_dimension: self.device.limits().max_texture_dimension_2d,
        }
    }

    /// Uploads the pixels an [`ImageDraw`](specular_scene::ImageDraw) with
    /// this id shows, replacing any earlier upload. `rgba` is tightly packed
    /// sRGB with straight alpha, `size.width * 4` bytes per row.
    ///
    /// This prepares the texels on the calling thread. For a large image,
    /// build the [`ImageMips`] elsewhere and call
    /// [`set_image_mips`](Self::set_image_mips).
    pub fn set_image(
        &mut self,
        id: ImageId,
        size: PixelSize,
        rgba: &[u8],
    ) -> Result<(), FrameImportError> {
        let mips = ImageMips::build(size, rgba, self.image_spec())?;
        self.set_image_mips(id, &mips)
    }

    /// Uploads an image prepared with this compositor's
    /// [`image_spec`](Self::image_spec), replacing any earlier upload under
    /// `id`. Drawn smaller than its size, it is sampled from the mip levels.
    pub fn set_image_mips(
        &mut self,
        id: ImageId,
        mips: &ImageMips,
    ) -> Result<(), FrameImportError> {
        let limit = self.device.limits().max_texture_dimension_2d;
        upload::validate_frame_size(mips.size(), limit)?;
        let srgb = self.target_format.is_srgb();
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("image-texture"),
            size: upload::extent(mips.size()),
            mip_level_count: mips.level_count(),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: upload::page_texture_format(PixelFormat::Rgba8Unorm, srgb),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, (size, texels)) in mips.levels().enumerate() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                texels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size.width * 4),
                    rows_per_image: None,
                },
                upload::extent(size),
            );
        }
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
