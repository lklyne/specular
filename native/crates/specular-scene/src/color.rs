//! Colours as the designer wrote them: 8-bit sRGB with straight alpha.

/// An sRGB colour with straight (not premultiplied) alpha.
///
/// Eight bits per channel so a scene snapshot reads like the CSS value it
/// came from. The renderer converts to whatever its target needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Opacity, 255 is opaque.
    pub a: u8,
}

impl Color {
    /// Fully transparent.
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    /// Opaque black.
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// Opaque white.
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// An opaque colour.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    /// A colour with alpha.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// The same colour at another alpha.
    #[must_use]
    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }
}
