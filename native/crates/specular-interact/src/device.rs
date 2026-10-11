//! The device a page tells its document it is on, from the device its
//! preset names: what Chrome's device mode sends for the same device.

use specular_core::{PageDevice, PageUserAgent};
use specular_doc::{EntityId, Page};

use crate::App;
use crate::app::page_of;

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

const ANDROID: PageUserAgent = PageUserAgent {
    user_agent: "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/138.0.0.0 Mobile Safari/537.36",
    navigator_platform: "Linux armv81",
    hint_platform: "Android",
    model: "K",
};

const fn handheld(user_agent: PageUserAgent, pixel_ratio: f32) -> PageDevice {
    PageDevice {
        user_agent: Some(user_agent),
        mobile: true,
        touch: true,
        pixel_ratio: Some(pixel_ratio),
    }
}

impl App {
    /// The device the page `id` is on now: the one its preset names, or
    /// the browser as it is while the page fills the view, where it is a
    /// browser tab under a mouse whatever size it is stored at.
    pub fn page_device(&self, id: &EntityId) -> PageDevice {
        if crate::showing::fills(self, id) {
            return PageDevice::default();
        }
        (self.document.entity(id).and_then(page_of)).map_or_else(PageDevice::default, preset_device)
    }
}

/// Whether the preset of the page `id` names a device that is not the
/// browser as it is.
pub(crate) fn has_own(app: &App, id: &EntityId) -> bool {
    (app.document.entity(id).and_then(page_of))
        .is_some_and(|page| preset_device(page) != PageDevice::default())
}

/// The device the preset of `page` names. A laptop, a desktop and a page with a size of
/// its own are the browser as it is, at a pixel ratio the shell picks.
fn preset_device(page: &Page) -> PageDevice {
    let device = (page.metadata.as_ref())
        .and_then(|meta| meta.get("deviceId"))
        .and_then(serde_json::Value::as_str);
    match device {
        Some("iphone-se") => handheld(IPHONE, 2.0),
        Some(
            "iphone-14-pro" | "iphone-14-pro-max" | "iphone-duo-cover" | "iphone-duo-open"
            | "iphone-17-pro-max" | "iphone-17" | "iphone-air" | "iphone-16e",
        ) => handheld(IPHONE, 3.0),
        Some(
            "ipad-mini" | "ipad-pro-11" | "ipad-pro-129" | "ipad-pro-13" | "ipad-pro-11-m4"
            | "ipad-air-11",
        ) => handheld(IPAD, 2.0),
        Some("pixel-10-pro-xl" | "galaxy-s25") => handheld(ANDROID, 3.0),
        // 1080 device pixels across 412.
        Some("pixel-10") => handheld(ANDROID, 2.625),
        _ => PageDevice::default(),
    }
}
