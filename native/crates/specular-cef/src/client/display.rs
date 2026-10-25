//! The display handler: titles, cursors, favicons, console lines.

use std::os::raw::c_int;

use cef::rc::Rc as _;
use cef::{
    Browser, CefString, CefStringList, CursorInfo, CursorType, DisplayHandler,
    DownloadImageCallback, Frame, Image, ImplBinaryValue, ImplBrowser, ImplBrowserHost,
    ImplDisplayHandler, ImplDownloadImageCallback, ImplFrame, ImplImage, WrapDisplayHandler,
    WrapDownloadImageCallback, wrap_display_handler, wrap_download_image_callback,
};
use specular_core::{Cursor, PageEvent};

use crate::page::PageContext;

wrap_display_handler! {
    pub(super) struct PageDisplayHandler {
        ctx: PageContext,
    }

    impl DisplayHandler {
        fn on_address_change(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            // A subframe's address is not the page's.
            if frame.is_some_and(|frame| frame.is_main() != 0) {
                self.ctx.push(PageEvent::Url {
                    page: self.ctx.id,
                    url: url.map(ToString::to_string).unwrap_or_default(),
                });
            }
        }

        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            // The first word of a new document, long before it has loaded:
            // early enough to lay it out once.
            if let Some(host) = browser.and_then(|browser| browser.host()) {
                self.ctx.pending.ask_viewport_fit(&host);
            }
            self.ctx.push(PageEvent::Title {
                page: self.ctx.id,
                title: title.map(ToString::to_string).unwrap_or_default(),
            });
        }

        fn on_cursor_change(
            &self,
            _browser: Option<&mut Browser>,
            _cursor: *mut u8,
            type_: CursorType,
            _custom_cursor_info: Option<&CursorInfo>,
        ) -> c_int {
            self.ctx.push(PageEvent::Cursor {
                page: self.ctx.id,
                cursor: cursor_of(type_),
            });
            // The page is drawn offscreen, so there is no view of CEF's to
            // set a cursor on.
            1
        }

        fn on_favicon_urlchange(
            &self,
            browser: Option<&mut Browser>,
            icon_urls: Option<&mut CefStringList>,
        ) {
            // The list is CEF's own, and only one taken whole can be read.
            let first = icon_urls.and_then(|urls| std::mem::take(urls).into_iter().next());
            let host = browser.and_then(|browser| browser.host());
            let (Some(url), Some(host)) = (first, host) else {
                self.ctx.push(PageEvent::Favicon {
                    page: self.ctx.id,
                    png: None,
                });
                return;
            };
            let mut fetched = FaviconFetched::new(self.ctx.clone());
            let url = CefString::from(url.as_str());
            host.download_image(Some(&url), 1, FAVICON_SIZE, 0, Some(&mut fetched));
        }
    }
}

/// The cursor to show for one a page asks for. A kind the shell has no
/// cursor for is the arrow.
fn cursor_of(kind: CursorType) -> Cursor {
    use cef::sys::cef_cursor_type_t as Kind;
    match *kind.as_ref() {
        Kind::CT_HAND => Cursor::Pointer,
        Kind::CT_IBEAM => Cursor::Text,
        Kind::CT_VERTICALTEXT => Cursor::VerticalText,
        Kind::CT_CROSS | Kind::CT_CELL => Cursor::Crosshair,
        Kind::CT_GRAB => Cursor::Grab,
        Kind::CT_GRABBING => Cursor::Grabbing,
        Kind::CT_MOVE => Cursor::Move,
        Kind::CT_NORTHWESTRESIZE | Kind::CT_SOUTHEASTRESIZE | Kind::CT_NORTHWESTSOUTHEASTRESIZE => {
            Cursor::ResizeNwse
        }
        Kind::CT_NORTHEASTRESIZE | Kind::CT_SOUTHWESTRESIZE | Kind::CT_NORTHEASTSOUTHWESTRESIZE => {
            Cursor::ResizeNesw
        }
        Kind::CT_EASTRESIZE | Kind::CT_WESTRESIZE | Kind::CT_EASTWESTRESIZE => Cursor::ResizeEw,
        Kind::CT_NORTHRESIZE | Kind::CT_SOUTHRESIZE | Kind::CT_NORTHSOUTHRESIZE => Cursor::ResizeNs,
        Kind::CT_COLUMNRESIZE => Cursor::ResizeColumn,
        Kind::CT_ROWRESIZE => Cursor::ResizeRow,
        Kind::CT_NOTALLOWED | Kind::CT_NODROP => Cursor::NotAllowed,
        Kind::CT_ALIAS => Cursor::Alias,
        Kind::CT_COPY => Cursor::Copy,
        Kind::CT_CONTEXTMENU => Cursor::ContextMenu,
        _ => Cursor::Default,
    }
}

/// The largest side of a page's icon, in pixels: a tab's glyph on a display
/// of twice the density, and as much again.
const FAVICON_SIZE: u32 = 64;

wrap_download_image_callback! {
    struct FaviconFetched {
        ctx: PageContext,
    }

    impl DownloadImageCallback {
        fn on_download_image_finished(
            &self,
            _image_url: Option<&CefString>,
            _http_status_code: c_int,
            image: Option<&mut Image>,
        ) {
            // CEF answers nothing to a caller that does not take the size.
            let (mut width, mut height) = (0, 0);
            let png = image
                .and_then(|image| image.as_png(1.0, 1, Some(&mut width), Some(&mut height)))
                .map(|png| {
                    let mut bytes = vec![0; png.size()];
                    let read = png.data(Some(&mut bytes), 0);
                    bytes.truncate(read);
                    bytes
                })
                .filter(|bytes| !bytes.is_empty());
            self.ctx.push(PageEvent::Favicon {
                page: self.ctx.id,
                png: png.map(Into::into),
            });
        }
    }
}
