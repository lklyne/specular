//! Keeping hosted pages in step with the document: after a change that can
//! add, remove or resize pages, the difference becomes page effects.

use std::collections::HashMap;

use specular_core::{CssSize, PageNav};
use specular_doc::{ColorScheme, EntityId};

use crate::app::page_of;
use crate::{App, Effect, PagePlacement};

/// What a host needs to know about one page entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostedPage {
    id: EntityId,
    url: String,
    viewport: CssSize,
    scheme: Option<ColorScheme>,
}

/// The pages of the document, back-to-front, each at the viewport its host
/// is laid out at: its rect's size, or the size its tab presents it at.
pub(crate) fn snapshot(app: &App) -> Vec<HostedPage> {
    let presented = presentation(app);
    (app.document.entities())
        .filter_map(|entity| {
            let page = page_of(entity)?;
            let viewport = match &presented {
                Some((id, viewport)) if *id == entity.id => *viewport,
                Some(_) | None => PagePlacement::viewport_for(entity.rect),
            };
            Some(HostedPage {
                id: entity.id.clone(),
                url: page.url.clone(),
                viewport,
                scheme: page.color_scheme,
            })
        })
        .collect()
}

/// The page a tab lays out at a size of its own, and that size.
pub(crate) fn presentation(app: &App) -> Option<(EntityId, CssSize)> {
    let entity = app.document.entity(app.shown_item()?)?;
    page_of(entity)?;
    let rect = crate::showing::presented_rect(app, entity)?;
    Some((entity.id.clone(), PagePlacement::viewport_for(rect)))
}

/// Appends the viewport changes that take the hosts from the page `before`
/// presented to the one presented now: the page that was laid out at a
/// tab's size goes back to its stored viewport, and the one that is now
/// takes the tab's. A lens, a tab, the window and the sidebar all change
/// it, so `update` asks once after every event.
pub(crate) fn follow_presentation(
    before: Option<&(EntityId, CssSize)>,
    app: &App,
    effects: &mut Vec<Effect>,
) {
    let now = presentation(app);
    if now.as_ref() == before {
        return;
    }
    let stays = |page: &EntityId| now.as_ref().is_some_and(|(shown, _)| shown == page);
    if let Some((page, _)) = before
        && !stays(page)
        && let Some(placement) = app.page_placement(page)
    {
        effects.push(Effect::SetPageViewport {
            page: page.clone(),
            viewport: placement.viewport,
        });
    }
    if let Some((page, viewport)) = now {
        effects.push(Effect::SetPageViewport { page, viewport });
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
