//! The two kinds of node that are a page: a `link` node, and a `file` node
//! whose file is HTML, which is shown live as Electron's `specular.html`
//! renderer shows it. The file node keeps a page's fields beside its path,
//! so a preset, a device frame, a sync set or a colour scheme is set on it
//! as on a link, and it stays a file node in the `.canvas`.

use std::borrow::Cow;

use crate::{FileRef, Kind, Page};

impl FileRef {
    /// Whether the file is shown as a live page: Electron's
    /// `HTML_EXTENSIONS`.
    pub fn is_html(&self) -> bool {
        self.file.rsplit_once('.').is_some_and(|(_, extension)| {
            extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
        })
    }
}

impl Kind {
    /// The page fields of a page node, or `None` for any other. An HTML
    /// file's `url` is empty: its address is its path, which only whoever
    /// knows the space folder can turn into a URL.
    pub fn page_fields(&self) -> Option<Cow<'_, Page>> {
        match self {
            Self::Page(page) => Some(Cow::Borrowed(page)),
            Self::File(file) if file.is_html() => Some(Cow::Owned(Page {
                url: String::new(),
                preset_index: file.preset_index,
                sync_id: file.sync_id.clone(),
                source: None,
                color_scheme: file.color_scheme,
                metadata: file.metadata.clone(),
            })),
            Self::File(_) | Self::Text(_) | Self::Group(_) | Self::Drawing(_) | Self::Shape(_) => {
                None
            }
        }
    }

    /// This page node with `page`'s fields, or `None` when it is no page.
    /// A link takes them whole. An HTML file takes all but the URL and the
    /// source, so it keeps naming its file.
    pub fn with_page_fields(&self, page: Page) -> Option<Self> {
        match self {
            Self::Page(_) => Some(Self::Page(page)),
            Self::File(file) if file.is_html() => Some(Self::File(FileRef {
                preset_index: page.preset_index,
                sync_id: page.sync_id,
                color_scheme: page.color_scheme,
                metadata: page.metadata,
                ..file.clone()
            })),
            Self::File(_) | Self::Text(_) | Self::Group(_) | Self::Drawing(_) | Self::Shape(_) => {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Document, Kind};

    #[test]
    fn an_html_file_keeps_a_pages_fields_and_stays_a_file_node() {
        let text = r#"{"nodes":[{"id":"h1","type":"file","file":"assets/a.html",
            "x":0,"y":0,"width":393,"height":852,"presetIndex":3,
            "metadata":{"showDeviceFrame":true,"deviceId":"iphone-14-pro"},
            "specular":{"syncId":"s1","colorScheme":"dark"}}],"edges":[]}"#;
        let document = Document::from_canvas_str(text).unwrap();
        let entity = document.entity(&"h1".into()).unwrap();
        let page = entity.kind.page_fields().unwrap();
        assert_eq!(page.sync_id.as_deref(), Some("s1"));
        assert_eq!(page.preset_index, Some(3));
        assert!(page.shell().is_some(), "its device frame is shown");
        let written = document.to_canvas_string().unwrap();
        let again = Document::from_canvas_str(&written).unwrap();
        assert_eq!(again, document, "{written}");
        assert!(matches!(
            entity.kind.with_page_fields(page.into_owned()),
            Some(Kind::File(_))
        ));
    }
}
