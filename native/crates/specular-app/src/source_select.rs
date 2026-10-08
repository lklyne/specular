//! Creates the page backend chosen on the command line.

use specular_core::PageSource;

use crate::cli::SourceKind;

/// CEF subprocess entry; `None` in the browser process (and always without CEF).
pub fn run_subprocess_if_needed() -> Option<i32> {
    #[cfg(feature = "cef")]
    {
        specular_cef::run_subprocess_if_needed()
    }
    #[cfg(not(feature = "cef"))]
    {
        None
    }
}

/// What the process around the backend looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Host {
    /// A window, whose event loop owns the main thread.
    Window,
    /// No window and no event loop: the caller pumps the backend.
    Headless,
}

/// The backend for `kind`; `Cef` fails on builds without the `cef` feature.
pub(crate) fn create_source(kind: SourceKind, host: Host) -> anyhow::Result<Box<dyn PageSource>> {
    match kind {
        SourceKind::Synthetic => {
            if host == Host::Window {
                tracing::warn!("synthetic CPU frames are not representative; never compare them");
            }
            Ok(Box::new(specular_core::SyntheticPageSource::new()))
        }
        SourceKind::Cef => create_cef_source(host),
    }
}

/// The port CDP clients reach the pages on: 9222 when it is free, so tools
/// find it where they expect, and otherwise one the system picks, so a
/// second app still starts.
#[cfg(feature = "cef")]
fn debugging_port() -> Option<u16> {
    use std::net::{Ipv4Addr, TcpListener};
    let free = |port: u16| {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).ok()?;
        Some(listener.local_addr().ok()?.port())
    };
    free(9222).or_else(|| free(0))
}

#[cfg(feature = "cef")]
fn create_cef_source(host: Host) -> anyhow::Result<Box<dyn PageSource>> {
    let config = specular_cef::CefConfig {
        remote_debugging_port: debugging_port(),
        cache_path: None,
        shared_texture: true,
        pump: match host {
            Host::Window => specular_cef::Pump::RunLoopTimer,
            Host::Headless => specular_cef::Pump::Caller,
        },
    };
    Ok(Box::new(specular_cef::CefPageSource::new(config)?))
}

#[cfg(not(feature = "cef"))]
fn create_cef_source(_host: Host) -> anyhow::Result<Box<dyn PageSource>> {
    anyhow::bail!("this build has no CEF support; rebuild with `--features cef`")
}
