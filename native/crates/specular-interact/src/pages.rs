//! Keeping hosted pages in step with the document: after a change that can
//! add, remove or resize pages, the difference becomes page effects.

use std::collections::HashMap;

use specular_core::{CssSize, PageNav};
use specular_doc::{ColorScheme, Document, EntityId};

use crate::app::page_of;
use crate::{Effect, PagePlacement};

/// What a host needs to know about one page entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostedPage {
    id: EntityId,
    url: String,
    viewport: CssSize,
    scheme: Option<ColorScheme>,
}

/// The pages of `document`, back-to-front.
pub(crate) fn snapshot(document: &Document) -> Vec<HostedPage> {
    document
        .entities()
        .filter_map(|entity| {
            Some(HostedPage {
                id: entity.id.clone(),
                url: page_of(entity)?.url.clone(),
                viewport: PagePlacement::viewport_for(entity.rect),
                scheme: page_of(entity)?.color_scheme,
            })
        })
        .collect()
}

/// Appends the effects that take the hosts from `before` to the pages
/// `document` holds now: closes first, then creates and viewport changes in
/// stack order. A page whose URL changed stays hosted and is sent there.
pub(crate) fn reconcile(before: &[HostedPage], document: &Document, effects: &mut Vec<Effect>) {
    let after = snapshot(document);
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
/// page of `document`: another canvas is being shown. A page is not kept
/// across the change even when both canvases have its id, as a duplicated
/// canvas does: its scroll and history belong to the canvas it was in.
pub(crate) fn replace(before: &[HostedPage], document: &Document, effects: &mut Vec<Effect>) {
    for page in before {
        effects.push(Effect::ClosePage(page.id.clone()));
    }
    reconcile(&[], document, effects);
}

fn color_scheme(page: &HostedPage) -> Effect {
    Effect::SetPageColorScheme {
        page: page.id.clone(),
        scheme: page.scheme,
    }
}

/// Appends the effect that tells every page of `document` the scheme it
/// follows, for when the app's appearance changed under pages with no
/// scheme of their own.
pub(crate) fn refresh_color_schemes(document: &Document, effects: &mut Vec<Effect>) {
    effects.extend(snapshot(document).iter().map(color_scheme));
}
