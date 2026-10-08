//! The GPUI pin, and what rides on it.
//!
//! `gpui-kit` and `gpui-pre` are pinned with `=` in `native/Cargo.toml`. The
//! kit names its GPUI snapshot exactly, and GPUI's API may change in any
//! snapshot, so nothing moves until the pin is moved on purpose.
//!
//! Three facts about GPUI's macOS backend hold the canvas up, and no API
//! promises any of them (ADR 0040, Risks):
//!
//! 1. GPUI's view is a subview of the window's content view, so the canvas
//!    view can go under it as a sibling.
//! 2. `raw_window_handle::HasWindowHandle for Window` returns that view, the
//!    one with GPUI's Metal layer.
//! 3. A window opened with `WindowBackgroundAppearance::Transparent` has a
//!    non-opaque Metal layer, so the canvas shows through what GPUI leaves
//!    unpainted.
//!
//! [`NativeCanvas::install`](crate::native::NativeCanvas::install) checks
//! all three on the live window at startup and fails the launch with the
//! fact that broke. The test below fails when the lockfile no longer has
//! the versions the facts were checked against.

/// The `gpui-kit` release the shell is written against.
pub(crate) const GPUI_KIT: &str = "0.7.1";
/// The GPUI snapshot that release is built on.
pub(crate) const GPUI_PRE: &str = "0.3.8";

/// The `cef` crate release, whose build number is Chromium's.
pub(crate) const CEF: &str = "154.3.0+154.0.32";

/// What the app is built from, for the settings dialog's About pane.
/// `backend` is the page backend this run hosts pages in.
pub(crate) fn about(backend: &str) -> Vec<specular_interact::AboutRow> {
    let row = |name: &str, version: &str| specular_interact::AboutRow {
        name: name.to_owned(),
        version: version.to_owned(),
    };
    vec![
        row("Specular Native", env!("CARGO_PKG_VERSION")),
        row("GPUI Kit", GPUI_KIT),
        row("GPUI", GPUI_PRE),
        row("CEF", CEF),
        row("Page backend", backend),
    ]
}

/// The pin, for an error message.
pub(crate) fn describe() -> String {
    format!(
        "checked against gpui-kit {GPUI_KIT} on gpui-pre {GPUI_PRE}; if the pin in \
         native/Cargo.toml moved, read ADR 0040's Risks and crates/specular-shell/src/pins.rs"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The versions of `package` in the workspace lockfile.
    fn locked(package: &str) -> Vec<String> {
        let lock = include_str!("../../../Cargo.lock");
        let header = format!("name = \"{package}\"");
        let mut lines = lock.lines();
        let mut versions = Vec::new();
        while let Some(line) = lines.next() {
            if line == header
                && let Some(version) = lines.next()
            {
                let version = version.trim_start_matches("version = ").trim_matches('"');
                versions.push(version.to_owned());
            }
        }
        versions
    }

    #[test]
    fn the_lockfile_has_the_pinned_versions_and_no_others() {
        assert_eq!(locked("gpui-kit"), [GPUI_KIT]);
        assert_eq!(locked("gpui-pre"), [GPUI_PRE]);
        assert_eq!(locked("cef"), [CEF]);
    }
}
