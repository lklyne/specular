//! A link the page opens in a new tab: `target=_blank`, `window.open`, a
//! Command-click or a middle click. The page is not let open a window of
//! its own; a new page goes on the canvas beside it instead, as Electron's
//! `duplicatePageFromSource` does for a link.
//!
//! The new page keeps the source's preset, size, device and colour scheme,
//! and drops what was keyed to the source's address (`overrides`). It joins
//! no sync set. A foreground open selects it and brings it into view; a
//! background one (Command-click, middle click) leaves the selection and
//! the camera where they are. Neither enters it.

use specular_doc::{Entity, EntityId, Kind, Page};

use crate::{App, Effect, geometry, live, update, verbs, zoom};

/// Whether `url` is an address a page can load: a link to `javascript:` or
/// to nothing opens no page.
fn loadable(url: &str) -> bool {
    let Some((scheme, rest)) = url.split_once(':') else {
        return false;
    };
    matches!(scheme, "http" | "https" | "file" | "data") && !rest.is_empty()
}

/// The page `source` asked to open `url` in a new tab.
pub(crate) fn open(
    app: &mut App,
    source: &EntityId,
    url: &str,
    foreground: bool,
    effects: &mut Vec<Effect>,
) {
    if !loadable(url) {
        return;
    }
    let Some(entity) = app.document.entity(source).cloned() else {
        return;
    };
    let Kind::Page(page) = &entity.kind else {
        return;
    };
    let mut metadata = page.metadata.clone().unwrap_or_default();
    metadata.remove("overrides");
    metadata.insert("createdFrom".to_owned(), "link".into());
    let page = Page {
        url: url.to_owned(),
        sync_id: None,
        metadata: Some(metadata),
        ..page.clone()
    };
    // Placed from what is seen, so a page in a device frame clears the
    // frame as well.
    let seen = crate::shown_rect(app, &entity).unwrap_or(entity.rect);
    let delta = verbs::free_spot(app, seen) - geometry::origin(seen);
    let id = EntityId::from(app.fresh_id().as_str());
    let rect = entity.rect.translated(delta.x, delta.y);
    let new = Entity::new(id.clone(), rect, Kind::Page(page));
    update::verb(app, effects, |app, effects| {
        live::create(app, new, effects);
        if foreground {
            app.session.selection.set([id.into()]);
            zoom::reveal(app, seen.translated(delta.x, delta.y));
        }
    });
}
