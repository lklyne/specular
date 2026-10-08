//! Pages: live web items on the canvas (JSON Canvas `link` nodes on disk).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::geometry::CssSize;
use crate::source::PageSourceError;

/// Identity of a page within one running [`PageSource`](crate::PageSource).
///
/// Allocated by the source on [`create_page`](crate::PageSource::create_page);
/// it is a process-local handle, not the persisted `.canvas` node id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PageId(pub u64);

impl fmt::Display for PageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "page#{}", self.0)
    }
}

/// What a [`PageSource`](crate::PageSource) needs to host a page.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSpec {
    /// Full URL including scheme and host.
    pub url: String,
    /// Layout viewport in CSS pixels (the page's breakpoint).
    pub viewport: CssSize,
    /// Texels per CSS pixel to raster at — the page's *texture scale*
    /// (CONTEXT.md, "Page textures"). Maps to CEF's device scale factor.
    pub texture_scale: f32,
    /// Target paint rate in frames per second (the page's *frame-rate LOD*).
    pub frame_rate: u32,
}

impl PageSpec {
    /// A spec at texture scale 1 and 60 fps.
    pub fn new(url: &str, viewport: CssSize) -> Self {
        Self {
            url: url.to_owned(),
            viewport,
            texture_scale: 1.0,
            frame_rate: 60,
        }
    }

    /// Rejects specs no source can host: an empty URL, an empty viewport, or
    /// a texture scale that is not positive and finite.
    pub fn validate(&self) -> Result<(), PageSourceError> {
        if self.url.trim().is_empty() {
            return Err(PageSourceError::InvalidSpec("empty URL".to_owned()));
        }
        validate_viewport(self.viewport)?;
        validate_texture_scale(self.texture_scale)
    }
}

/// Rejects a viewport with no area.
pub fn validate_viewport(viewport: CssSize) -> Result<(), PageSourceError> {
    if viewport.width == 0 || viewport.height == 0 {
        return Err(PageSourceError::InvalidSpec(format!(
            "empty viewport {}x{}",
            viewport.width, viewport.height
        )));
    }
    Ok(())
}

/// Rejects a texture scale (device scale factor) that is not a positive,
/// finite number.
pub fn validate_texture_scale(scale: f32) -> Result<(), PageSourceError> {
    if scale.is_finite() && scale > 0.0 {
        Ok(())
    } else {
        Err(PageSourceError::InvalidSpec(format!(
            "texture scale must be positive and finite, got {scale}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_with_blank_url_is_invalid() {
        let spec = PageSpec::new("  ", CssSize::new(10, 10));
        assert!(matches!(
            spec.validate(),
            Err(PageSourceError::InvalidSpec(_))
        ));

        {
            let spec = PageSpec::new("https://example.com/", CssSize::new(0, 10));
            assert!(spec.validate().is_err());
        }

        {
            let spec = PageSpec::new("https://example.com/", CssSize::new(1440, 900));
            assert!(spec.validate().is_ok());
        }

        {
            assert!(validate_texture_scale(f32::NAN).is_err());
        }

        {
            assert!(validate_texture_scale(f32::INFINITY).is_err());
        }

        {
            assert!(validate_texture_scale(0.0).is_err());
        }
    }
}
