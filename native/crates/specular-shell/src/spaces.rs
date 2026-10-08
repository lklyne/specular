//! The folder dialogs and the files Finder hands over.
//!
//! The app asks for a space folder through an effect and the runtime leaves
//! a note; this shows the dialog on GPUI's next turn and gives the runtime
//! the answer. A `.canvas` opened from Finder arrives as a `file://` URL,
//! before the window exists on a cold launch and at any time after.

use std::cell::RefCell;
use std::path::PathBuf;

use gpui_kit::{App, PathPromptOptions};

use crate::canvas;

thread_local! {
    /// The canvas file Finder asked for before there was a canvas to show
    /// it in.
    static WAITING: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Asks for a folder and opens it as the space. `create` only words the
/// dialog. A cancelled dialog changes nothing.
pub(crate) fn choose(create: bool, cx: &mut App) {
    let prompt = if create { "Create space" } else { "Open space" };
    let chosen = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some(prompt.into()),
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = chosen.await else {
            return;
        };
        let Some(folder) = paths.first() else {
            return;
        };
        canvas::with(|canvas| {
            canvas.runtime.choose_space(folder);
            canvas.refresh_models();
        });
        // The settings dialog names the space and is not drawn from the
        // models.
        cx.update(App::refresh_windows);
    })
    .detach();
}

/// The path a `file://` URL names, with its percent escapes undone.
fn file_path(url: &str) -> Option<PathBuf> {
    let path = url.strip_prefix("file://")?;
    let path = path.strip_prefix("localhost").unwrap_or(path);
    let mut bytes = Vec::with_capacity(path.len());
    let mut rest = path.bytes();
    while let Some(byte) = rest.next() {
        if byte == b'%' {
            let digits = [rest.next()?, rest.next()?];
            let digits = std::str::from_utf8(&digits).ok()?;
            bytes.push(u8::from_str_radix(digits, 16).ok()?);
        } else {
            bytes.push(byte);
        }
    }
    Some(PathBuf::from(String::from_utf8(bytes).ok()?))
}

/// Finder opened these with the app. The first `.canvas` file among them is
/// shown, in the space its folder is: now if the canvas is up, else once it
/// is.
pub(crate) fn opened_from_finder(urls: &[String]) {
    let file = (urls.iter().filter_map(|url| file_path(url))).find(|path| {
        path.extension()
            .is_some_and(|extension| extension == "canvas")
    });
    let Some(file) = file else {
        tracing::warn!(?urls, "nothing among these is a canvas file");
        return;
    };
    let shown = canvas::with(|canvas| {
        if let Err(error) = canvas.runtime.open_canvas_file(&file) {
            tracing::error!("{error:#}");
        }
        canvas.refresh_models();
    });
    if shown.is_none() {
        WAITING.with(|waiting| *waiting.borrow_mut() = Some(file));
    }
}

/// The canvas file Finder asked for before the window opened.
pub(crate) fn take_waiting() -> Option<PathBuf> {
    WAITING.with(|waiting| waiting.borrow_mut().take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_url_is_its_path_with_the_escapes_undone() {
        for (url, path) in [
            (
                "file:///Users/ada/Space/Home.canvas",
                Some("/Users/ada/Space/Home.canvas"),
            ),
            (
                "file:///Users/ada/My%20Space/caf%C3%A9.canvas",
                Some("/Users/ada/My Space/café.canvas"),
            ),
            ("file://localhost/tmp/a.canvas", Some("/tmp/a.canvas")),
            ("https://example.com/a.canvas", None),
            ("file:///tmp/bad%2", None),
        ] {
            assert_eq!(file_path(url), path.map(PathBuf::from), "{url}");
        }
    }
}
