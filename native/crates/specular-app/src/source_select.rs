//! Creates the page backend chosen on the command line.

use specular_core::PageSource;

use crate::cli::SourceKind;

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

/// The backend for `kind`; `Cef` fails on builds without the `cef` feature.
pub(crate) fn create_source(kind: SourceKind) -> anyhow::Result<Box<dyn PageSource>> {
    match kind {
        SourceKind::Synthetic => {
            tracing::warn!("synthetic CPU frames are not representative; never compare them");
            Ok(Box::new(specular_core::SyntheticPageSource::new()))
        }
        SourceKind::Cef => create_cef_source(),
    }
}

#[cfg(feature = "cef")]
fn create_cef_source() -> anyhow::Result<Box<dyn PageSource>> {
    let config = specular_cef::CefConfig {
        remote_debugging_port: Some(9222),
        cache_path: None,
        shared_texture: true,
    };
    Ok(Box::new(specular_cef::CefPageSource::new(config)?))
}

#[cfg(not(feature = "cef"))]
fn create_cef_source() -> anyhow::Result<Box<dyn PageSource>> {
    anyhow::bail!("this build has no CEF support; rebuild with `--features cef`")
}
