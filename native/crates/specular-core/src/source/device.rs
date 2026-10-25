//! The device a page is told it is on.

/// The `prefers-color-scheme` a page reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageColorScheme {
    /// `light`.
    Light,
    /// `dark`.
    Dark,
}

/// The user agent a page sends in place of the browser's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageUserAgent {
    /// The `User-Agent` header and `navigator.userAgent`.
    pub user_agent: &'static str,
    /// `navigator.platform`.
    pub navigator_platform: &'static str,
    /// The platform its client hints name (`Sec-CH-UA-Platform`).
    pub hint_platform: &'static str,
    /// The model its client hints name (`Sec-CH-UA-Model`).
    pub model: &'static str,
}

/// The device a page tells its document it is on. The default is the
/// browser as it is: a desktop whose pixel ratio is the texture scale.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PageDevice {
    /// The user agent it sends, as a mobile one. `None` is the browser's.
    pub user_agent: Option<PageUserAgent>,
    /// Whether it lays out as a phone does: `<meta viewport>` is followed,
    /// scrollbars overlay the content and text is sized for a small screen.
    pub mobile: bool,
    /// Whether it has a touch screen: `pointer: coarse`, `hover: none` and
    /// touch points.
    pub touch: bool,
    /// Its `devicePixelRatio`, whatever scale its texture is rastered at.
    /// `None` is the texture scale.
    pub pixel_ratio: Option<f32>,
    /// What its `env(safe-area-inset-*)` are. None for a page laid out
    /// inside its screen's safe area, which has nothing to keep clear of.
    pub safe_area: PageSafeArea,
}

/// How far in from each edge of its viewport a page is told the screen's
/// safe area starts, in CSS pixels: its `env(safe-area-inset-*)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PageSafeArea {
    /// `safe-area-inset-top`.
    pub top: u32,
    /// `safe-area-inset-right`.
    pub right: u32,
    /// `safe-area-inset-bottom`.
    pub bottom: u32,
    /// `safe-area-inset-left`.
    pub left: u32,
}
