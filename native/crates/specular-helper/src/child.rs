//! One Chromium child process, from framework load to exit code.

/// Runs the child this process was launched as and returns its exit code.
pub(crate) fn run() -> i32 {
    #[cfg(target_os = "macos")]
    if let Err(reason) = load_framework() {
        eprintln!("specular-helper: {reason}");
        return 1;
    }
    // Must precede every other CEF call, or the versioned C API resolves to
    // the wrong struct layouts.
    let _hash = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = cef::args::Args::new();
    cef::execute_process(Some(args.as_main_args()), None, std::ptr::null_mut())
}

/// Loads `Chromium Embedded Framework.framework` from the app bundle this
/// helper sits in. The helper executable is at
/// `App.app/Contents/Frameworks/App Helper (X).app/Contents/MacOS/`, three
/// levels below the `Frameworks/` folder that holds the framework. macOS
/// links CEF lazily, so nothing of CEF may run before this.
#[cfg(target_os = "macos")]
fn load_framework() -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;

    const FRAMEWORK: &str = "Chromium Embedded Framework.framework/Chromium Embedded Framework";

    let exe = std::env::current_exe().map_err(|err| format!("no executable path: {err}"))?;
    let folder = exe
        .parent()
        .ok_or_else(|| format!("{} has no parent folder", exe.display()))?;
    let path = folder.join("../../..").join(FRAMEWORK);
    let path = path
        .canonicalize()
        .map_err(|err| format!("no framework at {}: {err}", path.display()))?;
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| format!("{} has a NUL in it", path.display()))?;
    // SAFETY: `as_ptr` points at `c_path`'s NUL-terminated buffer, which is
    // live and unmodified for the whole `load_library` call; the binding only
    // reads it as a C string.
    let first_char = unsafe { &*c_path.as_ptr() };
    if cef::load_library(Some(first_char)) == 1 {
        Ok(())
    } else {
        Err(format!("could not load {}", path.display()))
    }
}
