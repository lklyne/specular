//! CEF process plumbing: subprocess entry, framework loading, API version.
//!
//! Chromium is multi-process. Every child (renderer, GPU, network/utility)
//! is a re-launch of an executable with `--type=<kind>`; on macOS the
//! children are the `<App> Helper*.app` bundles inside
//! `Contents/Frameworks/`. The spike uses one executable for all roles: the
//! bundling script copies `specular-app` into each helper bundle, and
//! [`run_subprocess_if_needed`] (first thing in `main`) diverts children into
//! `CefExecuteProcess` before any window or GPU setup happens.

use specular_core::PageSourceError;

use crate::config::is_subprocess;
use crate::error::CefError;

/// If this process was launched by CEF as a subprocess (renderer, GPU,
/// utility), runs it to completion and returns its exit code; returns `None`
/// in the browser process. Call first thing in `main`.
pub fn run_subprocess_if_needed() -> Option<i32> {
    let argv: Vec<String> = std::env::args().collect();
    if !is_subprocess(&argv) {
        return None;
    }
    #[cfg(target_os = "macos")]
    if let Err(err) = load_framework(true) {
        tracing::error!(%err, "CEF helper could not load the framework");
        return Some(1);
    }
    declare_api_version();
    let args = cef::args::Args::new();
    // Children get no `App`: the only customisation (command-line switches)
    // is applied in the browser process and Chromium forwards what children
    // need.
    Some(cef::execute_process(
        Some(args.as_main_args()),
        None,
        std::ptr::null_mut(),
    ))
}

/// Declares the CEF API version this binding was generated against. Must
/// precede every other CEF call in each process, or the versioned C API
/// entry points resolve to the wrong struct layouts.
pub(crate) fn declare_api_version() {
    let _hash = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
}

/// Loads `Chromium Embedded Framework.framework` relative to the running
/// executable. macOS links CEF lazily (the framework is not on the dyld
/// path), so nothing CEF-related may run before this.
#[cfg(target_os = "macos")]
pub(crate) fn load_framework(helper: bool) -> Result<(), CefError> {
    use std::os::unix::ffi::OsStrExt;

    let exe = std::env::current_exe().map_err(CefError::CurrentExe)?;
    let path = crate::config::framework_library_path(&exe, helper)
        .ok_or_else(|| CefError::FrameworkPath(exe.clone()))?;
    let path = path
        .canonicalize()
        .map_err(|source| CefError::FrameworkNotFound {
            path: path.clone(),
            source,
        })?;
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| CefError::FrameworkPath(path.clone()))?;
    // SAFETY: `as_ptr` points at `c_path`'s NUL-terminated buffer, which is
    // live and unmodified for the whole `load_library` call; the binding only
    // reads it as a C string.
    let first_char = unsafe { &*c_path.as_ptr() };
    if cef::load_library(Some(first_char)) == 1 {
        Ok(())
    } else {
        Err(CefError::LoadLibrary(path))
    }
}

/// Unloads the framework after `cef_shutdown`.
#[cfg(target_os = "macos")]
pub(crate) fn unload_framework() {
    if cef::unload_library() != 1 {
        tracing::warn!("cef_unload_library failed");
    }
}

/// Wraps a backend failure for the `PageSource` error type.
pub(crate) fn backend_error(err: CefError) -> PageSourceError {
    PageSourceError::Backend(Box::new(err))
}
