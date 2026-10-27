//! What a page is told of its screen and device, and asked of its target.

use serde_json::{Value, json};
use specular_core::{PageColorScheme, PageDevice};

use super::returned;

/// What the document says of the screen around it: whether its viewport
/// meta asks to cover the screen, and the colour a browser paints round it,
/// its `theme-color` or else its background. Answered once the document is
/// parsed, so a question put while it loads waits for that.
const VIEWPORT_FIT: &str = r#"
if (document.readyState === 'loading') {
  await new Promise((done) => document.addEventListener('DOMContentLoaded', done, { once: true }));
}
const metas = (name) => [...document.querySelectorAll('meta[name="' + name + '"]')];
const fit = metas('viewport').map((meta) => meta.content || '').join(',');
const rgb = (css) => {
  if (!css || !CSS.supports('color', css)) return null;
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 1;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) return null;
  context.fillStyle = css;
  context.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = context.getImageData(0, 0, 1, 1).data;
  if (a === 0) return null;
  // What shows through is the browser's white.
  return [r, g, b].map((c) => Math.round((c * a + 255 * (255 - a)) / 255));
};
const theme = metas('theme-color').find((meta) => !meta.media || matchMedia(meta.media).matches);
const background = (e) => (e ? rgb(getComputedStyle(e).backgroundColor) : null);
const color = (theme && rgb(theme.content))
  || background(document.body) || background(document.documentElement);
return JSON.stringify({ covers: /viewport-fit\s*=\s*cover/i.test(fit), color });
"#;

/// The message that asks a document how it fits its screen. The answer
/// waits for the document to be parsed.
pub(crate) fn viewport_fit_message(id: i32) -> Vec<u8> {
    json!({
        "id": id,
        "method": "Runtime.evaluate",
        "params": {
            "expression": format!("(async () => {{{VIEWPORT_FIT}}})()"),
            "returnByValue": true,
            "awaitPromise": true,
        },
    })
    .to_string()
    .into_bytes()
}

/// Whether the document covers its screen and the colour round it, as a
/// [`viewport_fit_message`] found them. `None` for an answer that cannot be
/// read, as that of a document that went away while it was asked.
pub(crate) fn parse_viewport_fit(result: &[u8]) -> Option<(bool, Option<[u8; 3]>)> {
    let found: Value = serde_json::from_str(&returned(result)?).ok()?;
    let covers = found.get("covers")?.as_bool()?;
    let color = (found.get("color").cloned())
        .and_then(|color| serde_json::from_value::<[u8; 3]>(color).ok());
    Some((covers, color))
}

/// The message that makes a page report `scheme` as its
/// `prefers-color-scheme`: `applyPageColorScheme`'s
/// `Emulation.setEmulatedMedia`, over the page's devtools channel.
pub(crate) fn color_scheme_message(id: i32, scheme: PageColorScheme) -> Vec<u8> {
    let value = match scheme {
        PageColorScheme::Light => "light",
        PageColorScheme::Dark => "dark",
    };
    json!({
        "id": id,
        "method": "Emulation.setEmulatedMedia",
        "params": { "features": [{ "name": "prefers-color-scheme", "value": value }] },
    })
    .to_string()
    .into_bytes()
}

/// The devtools methods that make a page tell its document it is on
/// `device`, each with its params: the ones Chrome's device mode sends.
/// Every one is sent whatever the device, so a page that was a phone stops
/// being one.
pub(crate) fn device_methods(device: &PageDevice) -> [(&'static str, Value); 4] {
    let user_agent = device.user_agent.map_or_else(
        // An empty user agent is the browser's own.
        || json!({ "userAgent": "" }),
        |agent| {
            json!({
                "userAgent": agent.user_agent,
                "platform": agent.navigator_platform,
                "userAgentMetadata": {
                    "platform": agent.hint_platform,
                    "platformVersion": "",
                    "architecture": "",
                    "model": agent.model,
                    "mobile": true,
                },
            })
        },
    );
    // No width or height: the page keeps the viewport it is laid out at.
    // Chromium rasters at the screen's scale whatever ratio is reported.
    let metrics = json!({
        "width": 0,
        "height": 0,
        "deviceScaleFactor": device.pixel_ratio.unwrap_or(0.0),
        "mobile": device.mobile,
    });
    let touch = json!({ "enabled": device.touch, "maxTouchPoints": 5 });
    // The insets `env(safe-area-inset-*)` resolve to. The `Max` ones are
    // what `env(safe-area-max-inset-*)` do, the same on a screen whose safe
    // area does not move.
    let area = device.safe_area;
    let safe_area = json!({ "insets": {
        "top": area.top, "topMax": area.top,
        "right": area.right, "rightMax": area.right,
        "bottom": area.bottom, "bottomMax": area.bottom,
        "left": area.left, "leftMax": area.left,
    }});
    [
        ("Emulation.setUserAgentOverride", user_agent),
        ("Emulation.setDeviceMetricsOverride", metrics),
        ("Emulation.setTouchEmulationEnabled", touch),
        ("Emulation.setSafeAreaInsetsOverride", safe_area),
    ]
}

/// The message that calls the devtools `method` with `params`.
pub(crate) fn method_message(id: i32, method: &str, params: &Value) -> Vec<u8> {
    json!({ "id": id, "method": method, "params": params })
        .to_string()
        .into_bytes()
}

/// The message that asks a page for its own devtools target.
pub(crate) fn target_info_message(id: i32) -> Vec<u8> {
    json!({ "id": id, "method": "Target.getTargetInfo" })
        .to_string()
        .into_bytes()
}
