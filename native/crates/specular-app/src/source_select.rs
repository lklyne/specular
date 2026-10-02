//! Picks the page backend for this build.

use specular_core::PageSource;

/// CEF subprocess entry; `None` in the browser process (and always without CEF).
pub(crate) fn run_subprocess_if_needed() -> Option<i32> {
    #[cfg(feature = "cef")]
    {
        specular_cef::run_subprocess_if_needed()
    }
    #[cfg(not(feature = "cef"))]
    {
        None
    }
}

/// The CEF source when built with `--features cef`, else the synthetic one.
#[cfg_attr(
    not(feature = "cef"),
    expect(clippy::unnecessary_wraps, reason = "CEF startup is fallible")
)]
pub(crate) fn create_source() -> anyhow::Result<Box<dyn PageSource>> {
    #[cfg(feature = "cef")]
    {
        let config = specular_cef::CefConfig {
            remote_debugging_port: Some(9222),
            cache_path: None,
            shared_texture: true,
        };
        Ok(Box::new(specular_cef::CefPageSource::new(config)?))
    }
    #[cfg(not(feature = "cef"))]
    {
        tracing::warn!("built without `cef`: synthetic CPU frames are not representative");
        Ok(Box::new(specular_core::SyntheticPageSource::new()))
    }
}
