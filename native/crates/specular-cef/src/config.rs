//! CEF startup options and the pure decisions derived from them.
//!
//! Everything here is compiled without the `cef` feature so it is unit-tested
//! on every platform; the CEF backend only copies these values into CEF
//! structs.

use std::path::{Path, PathBuf};

use specular_core::{PageSourceError, PageSpec};

/// Startup options for the CEF page source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CefConfig {
    /// Expose CDP on this port (`--remote-debugging-port`), so agent tools
    /// (agent-browser, Playwright `connectOverCDP`, raw CDP) can attach to
    /// every page, and the bench can drive input-latency probes.
    pub remote_debugging_port: Option<u16>,
    /// Profile/cache directory; `None` keeps the profile in memory.
    pub cache_path: Option<PathBuf>,
    /// Use `OnAcceleratedPaint` shared textures (`shared_texture_enabled`).
    /// Off forces `OnPaint` CPU frames, which are non-representative. Only
    /// honoured on macOS, the one platform whose shared handle (IOSurface)
    /// the compositor imports.
    pub shared_texture: bool,
}

impl CefConfig {
    /// Whether browsers should be created with `shared_texture_enabled` on
    /// the platform this was compiled for.
    pub fn uses_shared_texture(&self) -> bool {
        self.shared_texture && cfg!(target_os = "macos")
    }

    /// The `remote_debugging_port` value for `cef_settings_t`: CEF accepts
    /// 1024..=65535 and treats 0 as disabled.
    pub fn settings_debugging_port(&self) -> i32 {
        match self.remote_debugging_port {
            Some(port) if port >= 1024 => i32::from(port),
            _ => 0,
        }
    }
}

/// A Chromium command-line switch to append in the browser process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switch {
    /// Switch name without the leading `--`.
    pub name: &'static str,
    /// Value for `--name=value`, or `None` for a bare flag.
    pub value: Option<&'static str>,
}

/// Switches the browser process appends in `OnBeforeCommandLineProcessing`.
///
/// - `remote-allow-origins=*`: Chromium rejects CDP WebSocket upgrades that
///   carry an `Origin` header unless the origin is allow-listed, which breaks
///   browser-hosted agent tools.
/// - `use-mock-keychain` (macOS): otherwise the first cookie write in an
///   unsigned dev build pops a Keychain access prompt mid-benchmark.
pub fn browser_switches(config: &CefConfig) -> Vec<Switch> {
    let mut switches = Vec::new();
    if config.settings_debugging_port() != 0 {
        switches.push(Switch {
            name: "remote-allow-origins",
            value: Some("*"),
        });
    }
    if cfg!(target_os = "macos") {
        switches.push(Switch {
            name: "use-mock-keychain",
            value: None,
        });
    }
    switches
}

/// Frame-rate ceiling Specular asks for: 60 Hz displays, or 120 Hz on
/// high-refresh (`ProMotion`) displays.
pub const MAX_WINDOWLESS_FRAME_RATE: u32 = 120;

/// Clamps a requested page frame rate to what `SetWindowlessFrameRate`
/// accepts (minimum 1) and the spike's ceiling.
pub fn windowless_frame_rate(fps: u32) -> i32 {
    fps.clamp(1, MAX_WINDOWLESS_FRAME_RATE) as i32
}

/// Path of the CEF framework binary relative to the running executable
/// inside a macOS app bundle.
///
/// The browser executable lives in `App.app/Contents/MacOS/` and finds the
/// framework in `../Frameworks/`; helper executables live in
/// `App.app/Contents/Frameworks/App Helper (X).app/Contents/MacOS/` and walk
/// three levels up to reach the same `Frameworks/` directory.
pub fn framework_library_path(exe: &Path, helper: bool) -> Option<PathBuf> {
    const FRAMEWORK: &str = "Chromium Embedded Framework.framework/Chromium Embedded Framework";
    let up = if helper { "../../.." } else { "../Frameworks" };
    Some(exe.parent()?.join(up).join(FRAMEWORK))
}

/// Whether `args` (the process argv) belong to a CEF subprocess. Chromium
/// launches every child (renderer, GPU, utility) with `--type=<kind>`; the
/// browser process never has it.
pub fn is_subprocess<S: AsRef<str>>(args: &[S]) -> bool {
    args.iter().any(|arg| arg.as_ref().starts_with("--type="))
}

/// Rejects specs CEF cannot host: an empty URL, an empty viewport, or a
/// non-positive/non-finite device scale factor.
pub fn validate_spec(spec: &PageSpec) -> Result<(), PageSourceError> {
    if spec.url.trim().is_empty() {
        return Err(PageSourceError::InvalidSpec("empty URL".to_owned()));
    }
    if spec.viewport.width == 0 || spec.viewport.height == 0 {
        return Err(PageSourceError::InvalidSpec(format!(
            "empty viewport {}x{}",
            spec.viewport.width, spec.viewport.height
        )));
    }
    validate_scale(spec.texture_scale)
}

/// Rejects a device scale factor that is not a positive finite number.
pub fn validate_scale(scale: f32) -> Result<(), PageSourceError> {
    if scale.is_finite() && scale > 0.0 {
        Ok(())
    } else {
        Err(PageSourceError::InvalidSpec(format!(
            "texture scale must be positive and finite, got {scale}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use specular_core::CssSize;

    use super::*;

    #[test]
    fn spec_with_empty_url_is_invalid() {
        let spec = PageSpec::new(" ", CssSize::new(10, 10));
        assert!(matches!(
            validate_spec(&spec),
            Err(PageSourceError::InvalidSpec(_))
        ));
    }

    #[test]
    fn spec_with_zero_width_viewport_is_invalid() {
        let spec = PageSpec::new("https://example.com/", CssSize::new(0, 10));
        assert!(validate_spec(&spec).is_err());
    }

    #[test]
    fn nan_scale_is_invalid() {
        assert!(validate_scale(f32::NAN).is_err());
    }

    #[test]
    fn default_spec_is_valid() {
        let spec = PageSpec::new("https://example.com/", CssSize::new(1440, 900));
        assert!(validate_spec(&spec).is_ok());
    }

    #[test]
    fn debugging_port_below_1024_is_disabled() {
        let config = CefConfig {
            remote_debugging_port: Some(80),
            ..CefConfig::default()
        };
        assert_eq!(config.settings_debugging_port(), 0);
    }

    #[test]
    fn debugging_port_in_range_passes_through() {
        let config = CefConfig {
            remote_debugging_port: Some(9222),
            ..CefConfig::default()
        };
        assert_eq!(config.settings_debugging_port(), 9222);
    }

    #[test]
    fn remote_allow_origins_is_added_only_with_debugging() {
        let with = CefConfig {
            remote_debugging_port: Some(9222),
            ..CefConfig::default()
        };
        let has_allow = |config: &CefConfig| {
            browser_switches(config)
                .iter()
                .any(|switch| switch.name == "remote-allow-origins")
        };
        assert_eq!(
            (has_allow(&with), has_allow(&CefConfig::default())),
            (true, false)
        );
    }

    #[test]
    fn frame_rate_is_clamped_to_one_through_120() {
        assert_eq!(
            [0, 60, 120, 240].map(windowless_frame_rate),
            [1, 60, 120, 120]
        );
    }

    #[test]
    fn browser_framework_path_is_sibling_frameworks_dir() {
        let exe = Path::new("/A.app/Contents/MacOS/A");
        assert_eq!(
            framework_library_path(exe, false),
            Some(PathBuf::from(
                "/A.app/Contents/MacOS/../Frameworks/Chromium Embedded Framework.framework/Chromium Embedded Framework"
            ))
        );
    }

    #[test]
    fn helper_framework_path_walks_out_of_helper_bundle() {
        let exe = Path::new("/A.app/Contents/Frameworks/A Helper.app/Contents/MacOS/A Helper");
        assert_eq!(
            framework_library_path(exe, true),
            Some(PathBuf::from(
                "/A.app/Contents/Frameworks/A Helper.app/Contents/MacOS/../../../Chromium Embedded Framework.framework/Chromium Embedded Framework"
            ))
        );
    }

    #[test]
    fn type_switch_marks_a_subprocess() {
        assert!(is_subprocess(&["app", "--type=renderer", "--lang=en"]));
    }

    #[test]
    fn browser_process_args_are_not_a_subprocess() {
        assert!(!is_subprocess(&["app", "demo.canvas"]));
    }

    #[test]
    fn shared_texture_is_only_used_on_macos() {
        let config = CefConfig {
            shared_texture: true,
            ..CefConfig::default()
        };
        assert_eq!(config.uses_shared_texture(), cfg!(target_os = "macos"));
    }
}
