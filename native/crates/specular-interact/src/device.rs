//! The device a page tells its document it is on, from the device its
//! preset names: what Chrome's device mode sends for the same device.

use specular_core::{PageDevice, PageUserAgent};
use specular_doc::Page;

const IPHONE: PageUserAgent = PageUserAgent {
    user_agent: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 \
                 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1",
    navigator_platform: "iPhone",
    hint_platform: "iOS",
    model: "iPhone",
};

const IPAD: PageUserAgent = PageUserAgent {
    user_agent: "Mozilla/5.0 (iPad; CPU OS 18_5 like Mac OS X) AppleWebKit/605.1.15 \
                 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1",
    navigator_platform: "iPad",
    hint_platform: "iOS",
    model: "iPad",
};

const fn handheld(user_agent: PageUserAgent, pixel_ratio: f32) -> PageDevice {
    PageDevice {
        user_agent: Some(user_agent),
        mobile: true,
        touch: true,
        pixel_ratio: Some(pixel_ratio),
    }
}

/// The device `page` is on. A laptop, a desktop and a page with a size of
/// its own are the browser as it is, at a pixel ratio the shell picks.
pub fn page_device(page: &Page) -> PageDevice {
    let device = (page.metadata.as_ref())
        .and_then(|meta| meta.get("deviceId"))
        .and_then(serde_json::Value::as_str);
    match device {
        Some("iphone-se") => handheld(IPHONE, 2.0),
        Some("iphone-14-pro" | "iphone-14-pro-max" | "iphone-duo-cover" | "iphone-duo-open") => {
            handheld(IPHONE, 3.0)
        }
        Some("ipad-mini" | "ipad-pro-11" | "ipad-pro-129") => handheld(IPAD, 2.0),
        _ => PageDevice::default(),
    }
}
