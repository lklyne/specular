//! The CPU time this process has used, so a profile can say what its frames
//! cost outside the steps a frame times: a shell's own toolkit redrawing,
//! worker threads, the driver's threads.

use std::time::Duration;

/// User and system CPU time of this process so far, over every thread, or
/// `None` when the system would not say.
pub fn process_cpu_time() -> Option<Duration> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: `usage` points at writable storage for one `rusage`, which is
    // what `getrusage` fills in.
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return None;
    }
    // SAFETY: the struct is all integers, so the zeroed bytes were already a
    // valid value, and the successful call overwrote them with real ones.
    let usage = unsafe { usage.assume_init() };
    let seconds = |time: libc::timeval| {
        Duration::from_secs(u64::try_from(time.tv_sec).unwrap_or(0))
            + Duration::from_micros(u64::try_from(time.tv_usec).unwrap_or(0))
    };
    Some(seconds(usage.ru_utime) + seconds(usage.ru_stime))
}
