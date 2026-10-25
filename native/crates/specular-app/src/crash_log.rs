//! The error log: `~/Library/Logs/Specular/errors.log`, where the Electron
//! app writes its crashes too, so a field report has one place to look.
//!
//! Three things land there, each as `[time] kind` and a detail block:
//!
//! - a Rust panic (`panic`), with where it happened and a backtrace,
//! - every `tracing` event at ERROR, which is how a failed effect, a page
//!   whose process went away (`render-process-gone`) and a space that could
//!   not open are reported,
//! - nothing else: warnings stay on stderr.
//!
//! The file is capped at [`MAX_BYTES`], then moved aside to `errors.log.old`.
//! A write that fails gives up logging for the rest of the run, and a write
//! that fails while logging is not logged again, so a full disk cannot turn
//! into a loop. Native crash dumps are not written: CEF's Crashpad needs a
//! `crash_reporter.cfg` in the bundle, which it does not have yet.

use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// Overrides where the log goes, for a test or a run that must not touch
/// the real one.
pub(crate) const PATH_VARIABLE: &str = "SPECULAR_ERROR_LOG";
/// How large the file grows before it is moved aside.
const MAX_BYTES: u64 = 50 * 1024 * 1024;

/// One error log file.
#[derive(Debug)]
pub(crate) struct CrashLog {
    path: PathBuf,
    /// Set once a write has failed.
    disabled: AtomicBool,
    /// Set while a record is being written.
    writing: AtomicBool,
}

static LOG: OnceLock<CrashLog> = OnceLock::new();

impl CrashLog {
    pub(crate) fn at(path: PathBuf) -> Self {
        Self {
            path,
            disabled: AtomicBool::new(false),
            writing: AtomicBool::new(false),
        }
    }

    /// Appends one record.
    pub(crate) fn record(&self, kind: &str, detail: &str) {
        if self.disabled.load(Ordering::Relaxed) || self.writing.swap(true, Ordering::Acquire) {
            return;
        }
        let stamp = specular_interact::iso8601(unix_ms());
        let text = format!("[{stamp}] {kind}\n{detail}\n\n");
        if self.append(&text).is_err() {
            self.disabled.store(true, Ordering::Relaxed);
        }
        self.writing.store(false, Ordering::Release);
    }

    fn append(&self, text: &str) -> std::io::Result<()> {
        if let Some(folder) = self.path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        if std::fs::metadata(&self.path).is_ok_and(|meta| meta.len() >= MAX_BYTES) {
            let mut old = self.path.clone().into_os_string();
            old.push(".old");
            std::fs::rename(&self.path, old)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(text.as_bytes())
    }
}

fn unix_ms() -> u64 {
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    since.map_or(0, |since| {
        u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
    })
}

/// Where the log goes: the override, else the platform's logs folder.
fn path(variable: impl Fn(&str) -> Option<std::ffi::OsString>, macos: bool) -> Option<PathBuf> {
    let set = |name: &str| variable(name).filter(|value| !value.is_empty());
    if let Some(path) = set(PATH_VARIABLE) {
        return Some(PathBuf::from(path));
    }
    let home = set("HOME").map(PathBuf::from);
    if macos {
        return Some(home?.join("Library/Logs/Specular/errors.log"));
    }
    let state =
        (set("XDG_STATE_HOME").map(PathBuf::from)).or(home.map(|home| home.join(".local/state")));
    Some(state?.join("Specular/errors.log"))
}

/// Starts the log for this run: panics and ERROR events are written from
/// here on. Only a run with a window keeps one; a script or a snapshot
/// reports on stderr.
pub(crate) fn install() {
    let Some(path) = path(|name| std::env::var_os(name), cfg!(target_os = "macos")) else {
        return;
    };
    if LOG.set(CrashLog::at(path)).is_ok()
        && let Some(log) = LOG.get()
    {
        hook_panics(log);
    }
}

/// Writes every panic to `log`, then does what the hook before did.
pub(crate) fn hook_panics(log: &'static CrashLog) {
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let place = info.location().map_or_else(String::new, |at| {
            format!(" at {}:{}:{}", at.file(), at.line(), at.column())
        });
        let message = (info.payload().downcast_ref::<&str>().copied())
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("(no message)");
        let backtrace = std::backtrace::Backtrace::force_capture();
        let detail = format!(
            "thread '{}' panicked{place}:\n{message}\n{backtrace}",
            thread.name().unwrap_or("<unnamed>")
        );
        log.record("panic", &detail);
        before(info);
    }));
}

/// The `tracing` layer that copies ERROR events into the log, once it is
/// installed.
pub(crate) struct ErrorLayer;

impl<S: Subscriber> Layer<S> for ErrorLayer {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if *event.metadata().level() != Level::ERROR {
            return;
        }
        let Some(log) = LOG.get() else {
            return;
        };
        let mut fields = Fields::default();
        event.record(&mut fields);
        let kind = fields.message.as_deref().unwrap_or("error");
        let detail = format!("{}{}", event.metadata().target(), fields.rest);
        log.record(kind, &detail);
    }
}

/// An event's message and its other fields as ` key=value` pairs.
#[derive(Default)]
struct Fields {
    message: Option<String>,
    rest: String,
}

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        } else {
            let _ = write!(self.rest, " {}={value:?}", field.name());
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_owned());
        } else {
            let _ = write!(self.rest, " {}={value}", field.name());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    /// The variables set, whether it is macOS, and where the log goes.
    type Row = (
        &'static [(&'static str, &'static str)],
        bool,
        Option<&'static str>,
    );

    #[test]
    fn the_log_goes_where_the_variable_says_and_else_to_the_logs_folder() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                (pairs.iter().find(|(key, _)| *key == name)).map(|(_, value)| OsString::from(value))
            }
        };
        let rows: [Row; 4] = [
            (
                &[("HOME", "/Users/me")],
                true,
                Some("/Users/me/Library/Logs/Specular/errors.log"),
            ),
            (
                &[
                    ("HOME", "/Users/me"),
                    (PATH_VARIABLE, "/tmp/run/errors.log"),
                ],
                true,
                Some("/tmp/run/errors.log"),
            ),
            (
                &[("HOME", "/home/me")],
                false,
                Some("/home/me/.local/state/Specular/errors.log"),
            ),
            (&[], true, None),
        ];
        for (vars, macos, expected) in rows {
            assert_eq!(
                path(env(vars), macos),
                expected.map(PathBuf::from),
                "{vars:?}"
            );
        }
    }

    #[test]
    fn a_panic_is_written_to_the_log_with_its_message_and_place() {
        let dir = std::env::temp_dir().join(format!("specular-crash-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("Logs/errors.log");
        let log: &'static CrashLog = Box::leak(Box::new(CrashLog::at(file.clone())));
        hook_panics(log);
        let joined = std::thread::Builder::new()
            .name("doomed".to_owned())
            .spawn(|| panic!("the canvas fell over"))
            .unwrap()
            .join();
        let _ = std::panic::take_hook();
        assert!(joined.is_err());
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.starts_with('['), "{text}");
        assert!(
            text.contains("] panic\nthread 'doomed' panicked at "),
            "{text}"
        );
        assert!(text.contains("crash_log.rs"), "{text}");
        assert!(text.contains("the canvas fell over"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
