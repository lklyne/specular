//! Keeping hosted pages in step with the document: after a change that can
//! add, remove or resize pages, the difference becomes page effects.

use std::collections::HashMap;

use specular_core::{CssSize, PageNav};
use specular_doc::{ColorScheme, EntityId};

use crate::app::page_of;
use crate::{App, Effect, Gesture, PagePlacement};

/// What a host needs to know about one page entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostedPage {
    id: EntityId,
    url: String,
    viewport: CssSize,
    scheme: Option<ColorScheme>,
}

/// The pages of the document, back-to-front, each at the viewport its host
/// is laid out at: its rect's size, or the size its tab lays it out at.
pub(crate) fn snapshot(app: &App) -> Vec<HostedPage> {
    let laid_out = laid_out(app);
    (app.document.entities())
        .filter_map(|entity| {
            let page = page_of(entity)?;
            let viewport = (laid_out.iter().flatten())
                .find(|(id, _)| *id == entity.id)
                .map_or_else(
                    || PagePlacement::viewport_for(entity.rect),
                    |(_, size)| *size,
                );
            Some(HostedPage {
                id: entity.id.clone(),
                url: page.url.clone(),
                viewport,
                scheme: page.color_scheme,
            })
        })
        .collect()
}

/// A page a tab lays out at a size of its own, and that size.
pub(crate) type LaidOut = (EntityId, CssSize);

/// The page the tab showing presents at a size of its own.
fn presentation(app: &App) -> Option<LaidOut> {
    let entity = app.document.entity(app.shown_item()?)?;
    page_of(entity)?;
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

/// Appends the viewport changes an event made to the layouts `before` it.
pub(crate) fn follow_layouts(before: &Layouts, app: &App, effects: &mut Vec<Effect>) {
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
/// stack order. A page whose URL changed stays hosted and is sent there.
pub(crate) fn reconcile(before: &[HostedPage], app: &App, effects: &mut Vec<Effect>) {
    let after = snapshot(app);
    let was: HashMap<&EntityId, &HostedPage> = before.iter().map(|page| (&page.id, page)).collect();
    let is: HashMap<&EntityId, &HostedPage> = after.iter().map(|page| (&page.id, page)).collect();
    for page in before {
        if !is.contains_key(&page.id) {
            effects.push(Effect::ClosePage(page.id.clone()));
        }
    }
    for page in &after {
        match was.get(&page.id) {
            Some(old) => {
                if old.url != page.url {
                    effects.push(Effect::Navigate {
                        page: page.id.clone(),
                        nav: PageNav::To(page.url.clone()),
                    });
                }
                if old.scheme != page.scheme {
                    effects.push(color_scheme(page));
                }
                if old.viewport != page.viewport {
                    effects.push(Effect::SetPageViewport {
                        page: page.id.clone(),
                        viewport: page.viewport,
                    });
                }
            }
            // The shell gives a new page its scheme as it makes it.
            None => effects.push(Effect::CreatePage {
                page: page.id.clone(),
                url: page.url.clone(),
                viewport: page.viewport,
            }),
        }
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
