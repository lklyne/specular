//! Per-process physical footprint (macOS `proc_pid_rusage`).
//!
//! RSS misses what differs most between the shells: `IOSurface` and GPU
//! (`IOAccelerator`) allocations are charged to `phys_footprint`, not to
//! resident pages, and the Rust compositor keeps shared surfaces checked out
//! where Electron imports and copies them. `phys_footprint` is also what
//! Activity Monitor's "Memory" column shows.

/// Physical footprint of `pid` in bytes, or `None` when it cannot be read
/// (the process exited, belongs to another user, or this is not macOS).
#[cfg(target_os = "macos")]
pub(crate) fn phys_footprint(pid: u32) -> Option<u64> {
    let pid = libc::c_int::try_from(pid).ok()?;
    let mut info = std::mem::MaybeUninit::<libc::rusage_info_v4>::zeroed();
    // SAFETY: `info` points at writable storage for one `rusage_info_v4`,
    // exactly what the `RUSAGE_INFO_V4` flavour fills in.
    let status =
        unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V4, info.as_mut_ptr().cast()) };
    if status != 0 {
        return None;
    }
    // SAFETY: the struct is all integers, so the zeroed bytes were already a
    // valid value, and the successful call overwrote them with real ones.
    let info = unsafe { info.assume_init() };
    Some(info.ri_phys_footprint)
}

/// Physical footprint is a macOS measure; elsewhere there is none.
#[cfg(not(target_os = "macos"))]
pub(crate) fn phys_footprint(_pid: u32) -> Option<u64> {
    None
}
