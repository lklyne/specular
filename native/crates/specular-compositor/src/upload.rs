//! CPU frame upload (`Queue::write_texture`), the non-representative path.

use specular_core::{CpuFrame, PixelFormat, PixelRect, PixelSize};

/// The wgpu format for page texels. Page bytes are sRGB-encoded; sampling an
/// `*Srgb` view linearises them so an sRGB render target re-encodes once.
pub(crate) fn page_texture_format(format: PixelFormat, srgb: bool) -> wgpu::TextureFormat {
    match (format, srgb) {
        (PixelFormat::Bgra8Unorm, true) => wgpu::TextureFormat::Bgra8UnormSrgb,
        (PixelFormat::Bgra8Unorm, false) => wgpu::TextureFormat::Bgra8Unorm,
        (PixelFormat::Rgba8Unorm, true) => wgpu::TextureFormat::Rgba8UnormSrgb,
        (PixelFormat::Rgba8Unorm, false) => wgpu::TextureFormat::Rgba8Unorm,
    }
}

/// Checks a CPU frame before it reaches wgpu, whose validation failures
/// abort the process instead of returning an error.
pub(crate) fn validate_cpu_frame(frame: &CpuFrame, max_dimension: u32) -> Result<(), String> {
    let PixelSize { width, height } = frame.size;
    if frame.size.is_empty() {
        return Err(format!("empty frame {width}x{height}"));
    }
    if width > max_dimension || height > max_dimension {
        return Err(format!(
            "frame {width}x{height} exceeds the device limit of {max_dimension}"
        ));
    }
    let row_bytes = u64::from(width) * 4;
    if u64::from(frame.stride) < row_bytes {
        return Err(format!(
            "stride {} is shorter than a {width}px row",
            frame.stride
        ));
    }
    let needed = u64::from(frame.stride) * u64::from(height - 1) + row_bytes;
    if (frame.bgra.len() as u64) < needed {
        return Err(format!(
            "frame has {} bytes, {needed} needed",
            frame.bgra.len()
        ));
    }
    Ok(())
}

/// The regions of `frame` to upload, clamped to its bounds. An empty dirty
/// list (or `full`) means the whole frame.
pub(crate) fn upload_regions(frame: &CpuFrame, full: bool) -> impl Iterator<Item = PixelRect> + '_ {
    let whole = PixelRect::new(0, 0, frame.size.width, frame.size.height);
    let use_whole = full || frame.dirty.is_empty();
    let dirty: &[PixelRect] = if use_whole { &[] } else { &frame.dirty };
    use_whole.then_some(whole).into_iter().chain(
        dirty
            .iter()
            .filter_map(move |rect| clamp_rect(*rect, frame.size)),
    )
}

fn clamp_rect(rect: PixelRect, size: PixelSize) -> Option<PixelRect> {
    let left = i64::from(rect.x).max(0);
    let top = i64::from(rect.y).max(0);
    let right = (i64::from(rect.x) + i64::from(rect.width)).min(i64::from(size.width));
    let bottom = (i64::from(rect.y) + i64::from(rect.height)).min(i64::from(size.height));
    (right > left && bottom > top).then(|| {
        PixelRect::new(
            left as i32,
            top as i32,
            (right - left) as u32,
            (bottom - top) as u32,
        )
    })
}

/// Creates a sampled page texture of `size`.
pub(crate) fn create_page_texture(
    device: &wgpu::Device,
    size: PixelSize,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("page-cpu-texture"),
        size: extent(size),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

/// Writes the regions of a validated `frame` into `texture`.
pub(crate) fn write_frame(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    frame: &CpuFrame,
    full: bool,
) {
    for region in upload_regions(frame, full) {
        let offset = u64::from(frame.stride) * region.y as u64 + u64::from(region.x as u32) * 4;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x as u32,
                    y: region.y as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &frame.bgra,
            wgpu::TexelCopyBufferLayout {
                offset,
                bytes_per_row: Some(frame.stride),
                rows_per_image: None,
            },
            extent(region.size()),
        );
    }
}

/// A 2D extent of `size`.
pub(crate) fn extent(size: PixelSize) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: u32, height: u32, dirty: Vec<PixelRect>) -> CpuFrame {
        CpuFrame {
            size: PixelSize::new(width, height),
            stride: width * 4,
            bgra: vec![0; (width * height * 4) as usize],
            dirty,
        }
    }

    #[test]
    fn empty_dirty_list_uploads_whole_frame() {
        let regions: Vec<_> = upload_regions(&frame(8, 4, vec![]), false).collect();
        assert_eq!(regions, [PixelRect::new(0, 0, 8, 4)]);
    }

    #[test]
    fn full_upload_ignores_dirty_rects() {
        let frame = frame(8, 4, vec![PixelRect::new(1, 1, 2, 2)]);
        let regions: Vec<_> = upload_regions(&frame, true).collect();
        assert_eq!(regions, [PixelRect::new(0, 0, 8, 4)]);
    }

    #[test]
    fn dirty_rects_are_clamped_to_frame_bounds() {
        let frame = frame(8, 4, vec![PixelRect::new(-2, 2, 6, 10)]);
        let regions: Vec<_> = upload_regions(&frame, false).collect();
        assert_eq!(regions, [PixelRect::new(0, 2, 4, 2)]);
    }

    #[test]
    fn dirty_rects_outside_frame_are_skipped() {
        let frame = frame(8, 4, vec![PixelRect::new(20, 0, 4, 4)]);
        assert_eq!(upload_regions(&frame, false).count(), 0);
    }

    #[test]
    fn validate_rejects_short_buffer() {
        let mut frame = frame(8, 4, vec![]);
        frame.bgra.truncate(10);
        assert!(validate_cpu_frame(&frame, 8192).is_err());
    }

    #[test]
    fn validate_rejects_short_stride() {
        let mut frame = frame(8, 4, vec![]);
        frame.stride = 16;
        assert!(validate_cpu_frame(&frame, 8192).is_err());
    }

    #[test]
    fn validate_rejects_oversized_frame() {
        assert!(validate_cpu_frame(&frame(16, 4, vec![]), 8).is_err());
    }

    #[test]
    fn validate_accepts_padded_rows_without_trailing_padding() {
        let mut frame = frame(8, 4, vec![]);
        frame.stride = 40;
        frame.bgra = vec![0; 40 * 3 + 32];
        assert!(validate_cpu_frame(&frame, 8192).is_ok());
    }

    #[test]
    fn srgb_target_gets_srgb_page_format() {
        assert_eq!(
            page_texture_format(PixelFormat::Bgra8Unorm, true),
            wgpu::TextureFormat::Bgra8UnormSrgb
        );
    }
}
