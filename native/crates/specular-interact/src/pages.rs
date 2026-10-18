//! Keeping hosted pages in step with the document: after a change that can
//! add, remove or resize pages, the difference becomes page effects.
//!
//! Two kinds of entity are hosted: a page (a `link` node), and a file entity
//! whose file is HTML, which is shown live and takes input as Electron's
//! `specular.html` renderer does. The HTML file stays a `file` node, so
//! nothing a page reports about itself is written back to it.

use std::collections::HashMap;

use specular_core::{CssSize, PageDevice, PageNav};
use specular_doc::{ColorScheme, Entity, EntityId, Kind};

use crate::app::page_of;
use crate::{App, Effect, Gesture, PagePlacement};

/// Where a hosted page loads its document from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageAddress {
    /// A web address, loaded as it is.
    Url(String),
    /// An HTML file, by the path its file entity names. The shell resolves
    /// it against the space folder and loads it from disk, so the file's
    /// relative links find what sits next to it.
    File(String),
}

/// Whether `file` is shown as a live page: Electron's `HTML_EXTENSIONS`.
pub fn is_html_file(file: &str) -> bool {
    file.rsplit_once('.').is_some_and(|(_, extension)| {
        extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
    })
}

/// Where the page `entity` hosts loads from, or `None` when it hosts none.
pub(crate) fn address_of(entity: &Entity) -> Option<PageAddress> {
    match &entity.kind {
        Kind::Page(page) => Some(PageAddress::Url(page.url.clone())),
        Kind::File(file) if is_html_file(&file.file) => Some(PageAddress::File(file.file.clone())),
        Kind::File(_) | Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
    }
}

/// Whether `entity` is shown as a hosted page.
pub(crate) fn hosts_page(entity: &Entity) -> bool {
    match &entity.kind {
        Kind::Page(_) => true,
        Kind::File(file) => is_html_file(&file.file),
        Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => false,
    }
}

/// What a host needs to know about one page entity.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HostedPage {
    id: EntityId,
    address: PageAddress,
    viewport: CssSize,
    scheme: Option<ColorScheme>,
    device: PageDevice,
}

/// The pages of the document, back-to-front, each at the viewport its host
/// is laid out at: the size it is placed at, or the size its tab lays it
/// out at.
pub(crate) fn snapshot(app: &App) -> Vec<HostedPage> {
    let laid_out = laid_out(app);
    (app.document.entities())
        .filter_map(|entity| {
            let address = address_of(entity)?;
            let viewport = (laid_out.iter().flatten())
                .find(|(id, _)| *id == entity.id)
                .map(|(_, size)| *size)
                .or_else(|| Some(app.page_placement(&entity.id)?.viewport))?;
            Some(HostedPage {
                id: entity.id.clone(),
                address,
                viewport,
                scheme: page_of(entity).and_then(|page| page.color_scheme),
                device: app.page_device(&entity.id),
            })
        })
        .collect()
}

/// A page a tab lays out at a size of its own, and that size.
pub(crate) type LaidOut = (EntityId, CssSize);

/// The page the tab showing presents at a size of its own.
fn presentation(app: &App) -> Option<LaidOut> {
    let entity = app.document.entity(app.shown_item()?)?;
    if !hosts_page(entity) {
        return None;
    }
    let rect = crate::showing::presented_rect(app, entity)?;
    Some((entity.id.clone(), PagePlacement::viewport_for(rect)))
}

/// The pages whose hosts a tab lays out: the one presented, and the one
/// prepared for a tab that is pressed.
pub(crate) fn laid_out(app: &App) -> [Option<LaidOut>; 2] {
    [presentation(app), crate::showing::prepared(app)]
}

/// Appends the viewport changes that take the hosts from the pages tabs
/// laid out `before` to the ones they lay out now: a page that was laid out
/// at a tab's size goes back to its stored viewport, and one that is now
/// takes the tab's. A page prepared and then shown is already there. A
/// lens, a tab, a press on one, the window and the sidebar all change it,
/// so `update` asks once after every event.
fn follow_presentation(before: &[Option<LaidOut>; 2], app: &App, effects: &mut Vec<Effect>) {
    let now = laid_out(app);
    let size_in = |all: &[Option<LaidOut>; 2], page: &EntityId| {
        (all.iter().flatten()).find_map(|(id, size)| (id == page).then_some(*size))
    };
    for (page, _) in before.iter().flatten() {
        if size_in(&now, page).is_none()
            && let Some(placement) = app.page_placement(page)
        {
            effects.push(Effect::SetPageViewport {
                page: page.clone(),
                viewport: placement.viewport,
            });
        }
    }
    for (page, viewport) in now.iter().flatten() {
        if size_in(before, page) != Some(*viewport) {
            effects.push(Effect::SetPageViewport {
                page: page.clone(),
                viewport: *viewport,
            });
        }
    }
}

/// The viewports that follow the session and not a document step: the ones
/// tabs lay out, and the ones of the pages a handle drag is resizing.
pub(crate) type Layouts = ([Option<LaidOut>; 2], Vec<LaidOut>);

/// The session's layouts as they stand, for `follow_layouts` to compare
/// with after an event.
pub(crate) fn layouts(app: &App) -> Layouts {
    (laid_out(app), resizing(app))
}

/// Appends the device change that goes with a page starting or ceasing to
/// fill the view: there it is the browser as it is, and anywhere else the
/// device its preset names. A page whose preset names none is the same
/// device either way and is told nothing.
fn follow_device(before: Option<&EntityId>, app: &App, effects: &mut Vec<Effect>) {
    let now = presentation(app).map(|(page, _)| page);
    if before == now.as_ref() {
        return;
    }
    for page in before.into_iter().chain(&now) {
        if crate::device::has_own(app, page) {
            effects.push(Effect::SetPageDevice {
                page: page.clone(),
                device: app.page_device(page),
            });
        }
    }
}

/// Appends the viewport and device changes an event made to the layouts
/// `before` it.
pub(crate) fn follow_layouts(before: &Layouts, app: &App, effects: &mut Vec<Effect>) {
    follow_device(before.0[0].as_ref().map(|(page, _)| page), app, effects);
    follow_presentation(&before.0, app, effects);
    follow_resize(&before.1, app, effects);
}

/// The pages a handle drag is resizing, each at the viewport it is laid out
/// at.
fn resizing(app: &App) -> Vec<LaidOut> {
    let Some(Gesture::Resize(drag)) = &app.session.gesture else {
        return Vec::new();
    };
    (drag.starts().iter())
        .filter_map(|start| {
            let placement = app.page_placement(&start.id)?;
            Some((start.id.clone(), placement.viewport))
        })
        .collect()
}

/// Appends the viewport changes an event made to the pages a handle drag
/// was resizing `before` it: a page is laid out at its rect's size as the
/// handle moves, and at the size it started from when the drag is
/// abandoned. Like a window drag, nothing coalesces them.
fn follow_resize(before: &[LaidOut], app: &App, effects: &mut Vec<Effect>) {
    for (page, viewport) in before {
        if let Some(placement) = app.page_placement(page)
            && placement.viewport != *viewport
        {
            effects.push(Effect::SetPageViewport {
                page: page.clone(),
                viewport: placement.viewport,
            });
        }
    }
}

/// Appends the effects that take the hosts from `before` to the pages the
/// document holds now: closes first, then creates and viewport changes in
/// stack order. A page whose URL changed stays hosted and is sent there. A
/// host whose file changed is made again, since only the shell knows where
/// the file is.
pub(crate) fn reconcile(before: &[HostedPage], app: &App, effects: &mut Vec<Effect>) {
    let after = snapshot(app);
    let was: HashMap<&EntityId, &HostedPage> = before.iter().map(|page| (&page.id, page)).collect();
    let is: HashMap<&EntityId, &HostedPage> = after.iter().map(|page| (&page.id, page)).collect();
    let moved = |page: &HostedPage| {
        (is.get(&page.id)).is_some_and(|now| needs_new_host(&page.address, &now.address))
    };
    for page in before {
        if !is.contains_key(&page.id) || moved(page) {
            effects.push(Effect::ClosePage(page.id.clone()));
        }
    }
    for page in &after {
        match was.get(&page.id).filter(|old| !moved(old)) {
            Some(old) => {
                if let (PageAddress::Url(old_url), PageAddress::Url(url)) =
                    (&old.address, &page.address)
                    && old_url != url
                {
                    effects.push(Effect::Navigate {
                        page: page.id.clone(),
                        nav: PageNav::To(url.clone()),
                    });
                }
                if old.scheme != page.scheme {
                    effects.push(color_scheme(page));
                }
                if old.device != page.device {
                    effects.push(Effect::SetPageDevice {
                        page: page.id.clone(),
                        device: page.device,
                    });
                }
                if old.viewport != page.viewport {
                    effects.push(Effect::SetPageViewport {
                        page: page.id.clone(),
                        viewport: page.viewport,
                    });
                }
            }
            // The shell gives a new page its scheme and its device as it
            // makes it.
            None => effects.push(Effect::CreatePage {
                page: page.id.clone(),
                address: page.address.clone(),
                viewport: page.viewport,
            }),
        }
    }
}

/// Whether a host loaded from `old` has to be made again to load from
/// `new`: a URL is navigated to, and anything else is not.
fn needs_new_host(old: &PageAddress, new: &PageAddress) -> bool {
    match (old, new) {
        (PageAddress::Url(_), PageAddress::Url(_)) => false,
        _ => old != new,
    }
}

/// Appends the effects that close every page of `before` and host every
/// page of the document: another canvas is being shown. A page is not kept
/// across the change even when both canvases have its id, as a duplicated
/// canvas does: its scroll and history belong to the canvas it was in.
pub(crate) fn replace(before: &[HostedPage], app: &App, effects: &mut Vec<Effect>) {
    for page in before {
        effects.push(Effect::ClosePage(page.id.clone()));
    }
    reconcile(&[], app, effects);
}

fn color_scheme(page: &HostedPage) -> Effect {
    Effect::SetPageColorScheme {
        page: page.id.clone(),
        scheme: page.scheme,
    }
}

/// Appends the effect that tells every page of the document the scheme it
/// follows, for when the app's appearance changed under pages with no
/// scheme of their own.
pub(crate) fn refresh_color_schemes(app: &App, effects: &mut Vec<Effect>) {
    effects.extend(snapshot(app).iter().map(color_scheme));
}
