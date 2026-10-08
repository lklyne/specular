//! Copying a CEF `OnPaint` buffer into an owned [`CpuFrame`].
//!
//! `OnPaint`'s buffer is only valid during the callback, so every CPU frame
//! is copied out. This path is non-representative (ADR 0038: CPU frame paths
//! collapse under animation) and exists for platforms without the IOSurface
//! import and for `shared_texture: false` A/B runs.

use specular_core::{CpuFrame, PixelRect, PixelSize};

/// Why a paint buffer could not be copied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PaintBufferError {
    /// CEF reported a non-positive size.
    #[error("paint size {width}x{height} is empty")]
    Empty {
        /// Reported width.
        width: i32,
        /// Reported height.
        height: i32,
    },
    /// The buffer is shorter than `width * height * 4`.
    #[error("paint buffer has {actual} bytes, expected {expected}")]
    Short {
        /// Bytes required.
        expected: usize,
        /// Bytes provided.
        actual: usize,
    },
}

/// Byte length of a tightly packed BGRA image, or `None` if non-positive.
pub fn bgra_len(width: i32, height: i32) -> Option<usize> {
    let width = usize::try_from(width).ok().filter(|w| *w > 0)?;
    let height = usize::try_from(height).ok().filter(|h| *h > 0)?;
    width.checked_mul(height)?.checked_mul(4)
}

/// Copies a tightly packed BGRA `OnPaint` buffer (upper-left origin, stride
/// `width * 4`) and its dirty rects (texels) into an owned frame.
pub fn copy_paint(
    buffer: &[u8],
    width: i32,
    height: i32,
    dirty: &[PixelRect],
) -> Result<CpuFrame, PaintBufferError> {
    let expected = bgra_len(width, height).ok_or(PaintBufferError::Empty { width, height })?;
    let bytes = buffer.get(..expected).ok_or(PaintBufferError::Short {
        expected,
        actual: buffer.len(),
    })?;
    let size = PixelSize::new(width as u32, height as u32);
    let whole_frame = dirty.iter().any(|rect| {
        let right = i64::from(rect.x) + i64::from(rect.width);
        let bottom = i64::from(rect.y) + i64::from(rect.height);
        rect.x <= 0
            && rect.y <= 0
            && right >= i64::from(size.width)
            && bottom >= i64::from(size.height)
    });
    Ok(CpuFrame {
        size,
        stride: size.width * 4,
        bgra: bytes.to_vec(),
        dirty: if whole_frame {
            Vec::new()
        } else {
            dirty.to_vec()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_keeps_size_stride_and_bytes() {
        let buffer: Vec<u8> = (0..16).collect();
        let frame = copy_paint(&buffer, 2, 2, &[PixelRect::new(0, 0, 1, 1)]);
        assert_eq!(
            frame,
            Ok(CpuFrame {
                size: PixelSize::new(2, 2),
                stride: 8,
                bgra: buffer,
                dirty: vec![PixelRect::new(0, 0, 1, 1)],
            })
        );
    }

    #[test]
    fn full_frame_dirty_rect_collapses_to_all_dirty() {
        let frame = copy_paint(&[0; 16], 2, 2, &[PixelRect::new(0, 0, 2, 2)]);
        assert_eq!(frame.map(|f| f.dirty.is_empty()), Ok(true));
    }

    #[test]
    fn short_buffer_is_rejected() {
        assert_eq!(
            copy_paint(&[0; 15], 2, 2, &[]),
            Err(PaintBufferError::Short {
                expected: 16,
                actual: 15
            })
        );
    }
}
