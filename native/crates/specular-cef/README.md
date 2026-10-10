# specular-cef

The CEF offscreen-rendering (OSR) `PageSource` for the Rust CEF spike, plus
the CEF-free helpers it is built from. See `../../README.md` for the spike as
a whole and `docs/plans/rust-cef-spike.md` for what it measures.

## Layout

| Module | Feature | Responsibility |
|---|---|---|
| `config` | always | `CefConfig`, browser switches, frame-rate clamp, bundle-relative framework path, subprocess detection |
| `coords` | always | CSS -> texel rects, popup placement, IME bounds union (canvas -> page CSS lives in the app's `placement`) |
| `translate` | always | core `InputEvent` -> exact `CefBrowserHost` call (`HostCall`): flag bits, held buttons, wheel remainders, UTF-16 surrogates, IME range sentinels |
| `pool` | always | per-page cap on retained shared textures (`MAX_OUTSTANDING_TEXTURES` = 6) |
| `page` | always (`PageContext`: `cef`) | per-page view/popup geometry shared by handlers and the source |
| `cpu_frame` | always | `OnPaint` buffer -> owned `CpuFrame` |
| `dom_query` | always | the devtools messages that ask a page for the element under a point, the elements in a rect and its own target id, and reading the answers |
| `process` | `cef` | `run_subprocess_if_needed`, API-version declaration, macOS framework load/unload |
| `app_protocol` | `cef` + macOS | adds `CefAppProtocol` (`isHandlingSendEvent` / `setHandlingSendEvent:`) to winit's `NSApp` class before `cef_initialize` |
| `client` | `cef` | `wrap_*!` handler objects: app (switches), render, display, load, request, life-span, client |
| `devtools` | `cef` | one page's in-process devtools channel: `SendDevToolsMessage`, the message observer, answers -> `PageEvent` |
| `paint` | `cef` | `OnPaint` / `OnAcceleratedPaint` -> `FrameEvent` |
| `iosurface` | `cef` + macOS | the one unsafe IOSurface module: retain/use-count/release and its ownership rules |
| `source` | `cef` | `CefPageSource`: browser lifecycle, `PageSource` methods, `HostCall` dispatch |

Everything marked "always" is unit-tested on Linux in CI (`cargo test -p
specular-cef`). The `cef` modules are type-checked and linted on Linux and
for `aarch64-apple-darwin` with the `cef-dox` feature, which never downloads
or links CEF:

```sh
cd native
cargo clippy -p specular-cef --all-targets --features cef-dox -- -D warnings
cargo clippy -p specular-cef --all-targets --features cef-dox \
  --target aarch64-apple-darwin -- -D warnings
```

## How a page maps onto CEF

- **One windowless browser per page.** `WindowInfo.windowless_rendering_enabled`,
  Alloy runtime style, `GetViewRect` = the page's CSS viewport,
  `GetScreenInfo.device_scale_factor` = the page's texture scale. Frames
  arrive at viewport × scale texels. `set_viewport` and
  `set_texture_scale` only note the change; `flush_geometry` sends a
  page's as one `NotifyScreenInfoChanged` + `WasResized`.
- **Frame rate.** `BrowserSettings.windowless_frame_rate` at creation,
  `SetWindowlessFrameRate` later; clamped to 1..=120 (60 for regular
  displays, 120 for ProMotion).
- **Zero-copy frames (macOS).** `shared_texture_enabled = 1` ->
  `OnAcceleratedPaint` -> `PageFrame::GpuShared(SharedTexture)` carrying
  `NativeSurface::IoSurface`. The compositor imports it into wgpu
  (`newTextureWithDescriptor:iosurface:plane:` -> wgpu-hal Metal
  `texture_from_raw` -> `create_texture_from_hal`); this crate never touches
  Metal or wgpu.
- **Copy or retain.** CEF says the handle is only valid during the callback.
  The spike *retains* (`CFRetain` + `IOSurfaceIncrementUseCount`, undone
  when the compositor drops the `SharedTexture`), capped at 6 per page, the
  same model as Electron's `texture.release()`. The full reasoning, and the
  fallback if it tears, is in `src/iosurface.rs`.
- **CPU fallback.** Off macOS, or with `shared_texture: false`, `OnPaint`
  buffers are copied into `PageFrame::Cpu`. That path is **not
  representative** (ADR 0038) and its numbers never go in the results table.
- **Popups (the `<select>` case Electron OSR cannot do).** `OnPopupShow` ->
  `PageEvent::PopupVisibility`, `OnPopupSize` -> `PageEvent::PopupRect`
  (CSS rect scaled to texels, kept inside the view), and `PET_POPUP` paints
  -> `FrameLayer::Popup { rect }`. `GetScreenInfo` reports the view as the
  screen so Chromium keeps popups inside the page texture.
- **IME.** `ImeSetComposition` with explicit replacement/selection ranges
  (`CefRange::InvalidRange()` for "none", because a null pointer becomes
  `{0, 0}` on the C++ side), `ImeCommitText`, `ImeFinishComposingText`,
  `ImeCancelComposition`. `OnImeCompositionRangeChanged` ->
  `PageEvent::ImeCompositionBounds` for candidate-window placement.
- **Loads and crashes.** `OnLoadEnd` (main frame) -> `PageEvent::Loaded`;
  `OnRenderProcessTerminated` -> `PageEvent::Crashed` with Electron's
  `render-process-gone` reason strings (`crashed`, `oom`, `killed`,
  `launch-failed`).
- **Title, address, load and scroll.** `OnTitleChange` -> `PageEvent::Title`,
  `OnAddressChange` (main frame) -> `PageEvent::Url`, `OnLoadingStateChange`
  -> `PageEvent::Loading` with can-go-back and can-go-forward,
  `OnScrollOffsetChanged` -> `PageEvent::Scrolled` in CSS pixels.
- **Navigation.** `PageSource::navigate` keeps the browser: `Frame::LoadURL`
  on the main frame, `GoBack`, `GoForward`, `Reload`, `StopLoad`. A URL change
  never closes and recreates a page, so its history, focus and frames stay.
- **Questions about the DOM.** `query_element` and `query_elements_in_rect`
  send one `Runtime.evaluate` each over the page's in-process devtools
  channel (no socket, no debugging port needed) and the observer turns the
  result into `PageEvent::ElementAt` / `ElementsInRect`. The expressions
  follow the Electron preload's rules: an `nth-of-type` selector up to the
  nearest unique id, a four-segment readable path, and a region grabs the
  visible interactive elements it wholly contains, 15 at most.
- **`window.open`** is blocked (`OnBeforePopup` returns 1). Otherwise it would
  open a native window off the canvas.
- **Agents.** `CefConfig.remote_debugging_port` -> `cef_settings_t.remote_debugging_port`
  (1024..=65535) plus `--remote-allow-origins=*`, so agent-browser,
  Playwright `chromium.connectOverCDP("http://localhost:9222")` and raw CDP
  clients can attach. Every page shows up as a CDP target, and each page
  asks for its own target id at creation (`Target.getTargetInfo`) and
  reports it as `PageEvent::DevtoolsTarget`, so the app can hand an agent
  `ws://127.0.0.1:<port>/devtools/page/<id>` for one page. The app asks for
  port 9222 and takes a free one when that is in use.
- **One profile root per process.** CEF lets one process own a root cache
  path; a second launch on the same root passes itself to the first and
  exits, and that hand-off crashed the first (windowless) app. With
  `cache_path: None` each process gets `$TMPDIR/specular-cef-<pid>` as its
  root, keeps the profile in memory, and removes the folder on drop.
- **Message loop.** `multi_threaded_message_loop = 0`,
  `external_message_pump = 1`. On macOS a 240 Hz main-run-loop timer
  (`pump_timer`) calls `CefDoMessageLoopWork`: the call spins a nested
  run-loop turn, which panics winit if made inside one of its handlers.
  Elsewhere `PageSource::pump` calls it once per winit loop turn.
  `OnScheduleMessagePumpWork` is not used. A process with no event loop (the
  app's `--snapshot` and `--script` runs) sets `CefConfig.pump` to
  `Pump::Caller`: no timer, and `pump` makes the call itself.

## macOS setup (Apple Silicon), the representative configuration

1. **CEF binaries.** The first `--features cef` build downloads the CEF
   distribution (~300 MB) for cef crate 154.3 (CEF 154.0.32) from
   `cef-builds.spotifycdn.com`. Pin the location so builds and bundling
   share it:

   ```sh
   export CEF_PATH="$HOME/.local/share/cef"
   ```

2. **Build and bundle.** CEF on macOS only runs from an `.app` bundle: the
   framework is loaded from `Contents/Frameworks/` relative to the
   executable, and Chromium launches its children from the helper bundles.

   ```sh
   cd native
   cargo build -p specular-app --release --features cef
   crates/specular-cef/scripts/bundle-macos.sh release
   # -> target/release/specular-app.app
   target/release/specular-app.app/Contents/MacOS/specular-app path/to/file.canvas
   ```

   Run the inner binary directly (not `open`) to keep stdout/stderr and argv.
   `bundle-macos.sh` copies the one binary into five helper bundles
   (`specular-app Helper`, `Helper (GPU)`, `Helper (Renderer)`,
   `Helper (Plugin)`, `Helper (Alerts)`), copies
   `Chromium Embedded Framework.framework`, writes the `Info.plist`s
   (helpers get `LSUIElement`), and ad-hoc signs the bundle. It mirrors
   `cef::build_util::mac::bundle`. Rebuild means re-run the script: the
   bundle holds copies, not symlinks.

3. **Code signing.** Apple Silicon kills unsigned arm64 code, so the script
   runs `codesign --force --deep --sign -` (ad-hoc). That is enough locally.
   Distribution would need a Developer ID, hardened runtime, each helper
   signed separately inside-out (`--deep` is deprecated for that), and the
   entitlements CEF documents for helpers
   (`com.apple.security.cs.allow-jit`,
   `com.apple.security.cs.allow-unsigned-executable-memory`,
   `com.apple.security.cs.disable-library-validation`). The spike runs with
   `no_sandbox = 1` (the helper does not initialise `cef::sandbox::Sandbox`).

4. **Startup order in `main`.** `run_subprocess_if_needed()` first (helpers
   must not create windows). Then winit's `EventLoop::new()`, which installs
   winit's `NSApplication` subclass. Then `CefPageSource::new()`. CEF would
   otherwise create a plain `NSApplication` first, and winit panics when the
   principal class is not its own.

## CEF calls in use

Every CEF call below is type-checked against the cef 154.3 bindings
(`cef-dox`, Linux and `aarch64-apple-darwin`). A debug build has run them on
Apple Silicon, windowed and headless: startup and shutdown, shared-texture
paints, resize and rescale, pointer and wheel input, title, address, load and
scroll reports, navigation, and the devtools questions. Nobody has yet
watched key events, IME composition or a `<select>` popup in a window; those
are on the by-hand checklist in the run log.

- Process: `cef::api_hash(CEF_API_VERSION_LAST, 0)`, `cef::load_library`,
  `cef::unload_library` (macOS), `cef::args::Args::new`,
  `cef::execute_process(args, None, null)`,
  `cef::initialize(args, settings, app, null)`, `cef::do_message_loop_work`,
  `cef::shutdown`.
- `Settings`: `no_sandbox`, `windowless_rendering_enabled`,
  `external_message_pump`, `multi_threaded_message_loop`,
  `remote_debugging_port`, `cache_path`, `root_cache_path`,
  `background_color`.
- `App::on_before_command_line_processing` -> `CommandLine::append_switch`,
  `append_switch_with_value`.
- `cef::browser_host_create_browser_sync(window_info, client, url, settings, None, None)`
  with `WindowInfo { windowless_rendering_enabled, shared_texture_enabled,
  external_begin_frame_enabled: 0, runtime_style: ALLOY }` and
  `BrowserSettings { windowless_frame_rate, background_color }`;
  `Browser::host`.
- `BrowserHost`: `was_resized`, `notify_screen_info_changed`, `was_hidden`,
  `set_windowless_frame_rate`, `set_focus`, `close_browser(1)`,
  `send_mouse_move_event`, `send_mouse_click_event`,
  `send_mouse_wheel_event`, `send_key_event`, `ime_set_composition`
  (underlines `None`), `ime_commit_text` (`relative_cursor_pos` 0),
  `ime_finish_composing_text`, `ime_cancel_composition`.
- `RenderHandler`: `view_rect`, `screen_info`, `screen_point`,
  `on_popup_show`, `on_popup_size`, `on_paint`, `on_accelerated_paint`
  (`AcceleratedPaintInfo.shared_texture_io_surface`, `.format`,
  `.extra.coded_size`, `.extra.visible_rect`),
  `on_ime_composition_range_changed`.
- `DisplayHandler::on_address_change` / `on_title_change`,
  `LoadHandler::on_loading_state_change`,
  `RenderHandler::on_scroll_offset_changed`.
- `Browser::main_frame` + `Frame::load_url`, `Browser::go_back`,
  `go_forward`, `reload`, `stop_load`.
- `BrowserHost::add_dev_tools_message_observer` (the `Registration` is kept
  for the page's life), `send_dev_tools_message`,
  `DevToolsMessageObserver::on_dev_tools_method_result`.
- `LoadHandler::on_load_end` (`Frame::is_main`),
  `RequestHandler::on_render_process_terminated`,
  `LifeSpanHandler::on_before_popup` / `on_before_close`.
- `objc2-io-surface`: `IOSurfaceRef::increment_use_count` /
  `decrement_use_count`; `CFRetained::retain` on the raw IOSurface.
- `objc2` runtime: `class_addMethod`, `class_addProtocol` and the
  `cef::application_mac` protocols (`app_protocol`).

## Known risks to check first on a real run

Checked on a real run so far: CEF starts under winit's `NSApp` and under a
plain one (headless), pages paint through IOSurfaces with no import
failures, and two apps run side by side. Still open:

1. **`CefAppProtocol` on winit's `NSApp`.** Chromium sends
   `isHandlingSendEvent` / `setHandlingSendEvent:` to `NSApp`, and winit's
   `WinitApplication` implements neither. `app_protocol::install` adds both
   methods plus the `CrAppProtocol` / `CrAppControlProtocol` /
   `CefAppProtocol` conformances to winit's class before `cef_initialize`
   and fails startup with `CefError::AppProtocol` if that does not take.
   Type-checked only: confirm CEF starts and a `<select>` popup opens
   without an "unrecognized selector" crash.
2. **Retained IOSurface reuse.** Verify that Chromium does not repaint a
   surface whose use count we hold: no tearing under the `fast-pan-zoom`
   profile on an animated page. If it does, use the copy strategy in
   `src/iosurface.rs`.
   The compositor also caches one Metal/wgpu texture per surface
   (`specular-compositor`'s `import_cache`). If wrapping an IOSurface in a
   Metal texture marks it in use for Chromium's pool, the pool would grow
   instead of recycling: watch the `shared-surface import cache` line the
   app logs at exit (misses should stay near the pool size, not near the
   paint count) and the tree footprint.
3. **Coded vs visible size.** `SharedTexture` has no sub-rect. The paint
   path logs at `debug` when `visible_rect` differs from `coded_size`. If it
   happens, the compositor would sample padding.
4. **Frame rate > 60.** CEF documents only a minimum for
   `windowless_frame_rate`. Confirm 120 is honoured with shared textures, or
   switch to `external_begin_frame_enabled` + `SendExternalBeginFrame`
   driven from the display link.
