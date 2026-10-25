//! Creates the page backend chosen on the command line.

use std::path::Path;
#[cfg(feature = "cef")]
use std::path::PathBuf;

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
/// `profile` is the folder a CEF profile is kept in between launches;
/// without one it is in memory.
pub(crate) fn create_source(
    kind: SourceKind,
    host: Host,
    profile: Option<&Path>,
) -> anyhow::Result<Box<dyn PageSource>> {
    match kind {
        SourceKind::Synthetic => {
            if host == Host::Window {
                tracing::warn!("synthetic CPU frames are not representative; never compare them");
            }
            Ok(Box::new(specular_core::SyntheticPageSource::new()))
        }
        SourceKind::Cef => create_cef_source(host, profile),
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

/// Takes the profile folder for this process, or `None` when another
/// process has it. CEF allows one process a profile: a second one would
/// hand itself to the first and bring it down, so the second keeps its
/// profile in memory. The lock is held until the process ends.
#[cfg(feature = "cef")]
fn claim_profile(profile: &Path) -> Option<PathBuf> {
    static LOCK: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();
    let claimed = std::fs::create_dir_all(profile)
        .and_then(|()| std::fs::File::create(profile.join(".specular-lock")))
        .map_err(|error| error.to_string())
        .and_then(|file| {
            file.try_lock()
                .map(|()| file)
                .map_err(|error| error.to_string())
        });
    match claimed {
        Ok(file) => {
            let _ = LOCK.set(file);
            tracing::info!(folder = %profile.display(), "pages keep their profile");
            Some(profile.to_owned())
        }
        Err(error) => {
            tracing::warn!(
                folder = %profile.display(),
                "the page profile is in use or cannot be made ({error}); logins will not be kept"
            );
            None
        }
    }
}

#[cfg(feature = "cef")]
fn create_cef_source(host: Host, profile: Option<&Path>) -> anyhow::Result<Box<dyn PageSource>> {
    let config = specular_cef::CefConfig {
        remote_debugging_port: debugging_port(),
        cache_path: profile.and_then(claim_profile),
        shared_texture: true,
        pump: match host {
            Host::Window => specular_cef::Pump::RunLoopTimer,
            Host::Headless => specular_cef::Pump::Caller,
        },
    };
    Ok(Box::new(specular_cef::CefPageSource::new(config)?))
}

#[cfg(not(feature = "cef"))]
fn create_cef_source(_host: Host, _profile: Option<&Path>) -> anyhow::Result<Box<dyn PageSource>> {
    anyhow::bail!("this build has no CEF support; rebuild with `--features cef`")
}
