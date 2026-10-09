//! Element attachment (ADR 0032): which element of its page an anchored
//! item follows.
//!
//! The element is derived from placement, never chosen. After every step
//! that leaves an anchored entity somewhere new, its page is asked which
//! element is under the entity's centre, and the answer is stamped on the
//! anchor as `element`. A region comment on a page is asked for once, when
//! it is made. The stamp is written to the document with no undo step: the
//! user did not choose it, so it takes no place in the history, and an undo
//! that moves the entity asks again.
//!
//! Each page is also told which selectors its items follow, and reports
//! where those elements are as they move (`PageState::elements`). The
//! drawn position is corrected from that in `scroll_follow`.

use std::collections::{HashMap, HashSet};

use glam::DVec2;
use specular_core::CapturedElement;
use specular_doc::{
    AnchorElement, Annotation, AnnotationAnchor, AnnotationId, Command, Entity, EntityId, JsonMap,
    PageAnchor, Rect, RegionAnchor,
};

use crate::{App, Effect, Event, geometry, scroll_follow};

/// An anchored entity as it was when its element was last asked for.
#[derive(Debug, Clone, PartialEq)]
struct Placed {
    rect: Rect,
    page: EntityId,
    url: Option<String>,
}

impl Placed {
    fn of(entity: &Entity) -> Option<Self> {
        let anchor = entity.anchor.as_ref()?;
        Some(Self {
            rect: entity.rect,
            page: anchor.page_id.clone(),
            url: anchor.page_url.clone(),
        })
    }
}

/// What an element question is about.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Target {
    Entity(EntityId),
    Comment(AnnotationId),
}

/// The element questions put to pages, and what each page was told to
/// track. Session state: none of it is saved or undone.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Attachments {
    next: u64,
    /// Every anchored entity as it was when it was last asked about. One
    /// that has since moved or changed page is asked about again.
    placed: HashMap<EntityId, Placed>,
    /// The region comments that have been asked about, which happens once.
    comments: HashSet<AnnotationId>,
    /// The questions not yet answered. A newer question for the same item
    /// takes the older one out, so a late answer to it is dropped.
    asked: HashMap<u64, Target>,
    /// The selectors each page was last told to track, sorted.
    tracked: HashMap<EntityId, Vec<String>>,
    /// Whether an element was stamped since the pages were last told.
    stamped: bool,
}

impl Attachments {
    /// Whether `entity` is still to hear which element it follows: it has
    /// moved since it was last asked about, or its question is out.
    pub(crate) fn awaits(&self, entity: &Entity) -> bool {
        let asked = || (self.asked.values()).any(|it| *it == Target::Entity(entity.id.clone()));
        Placed::of(entity).is_some_and(|now| self.placed.get(&entity.id) != Some(&now) || asked())
    }

    /// Whether the region comment `annotation` is still to hear which
    /// element it follows.
    pub(crate) fn awaits_comment(&self, annotation: &Annotation) -> bool {
        let asked =
            || (self.asked.values()).any(|it| *it == Target::Comment(annotation.id.clone()));
        region_centre(annotation).is_some() && (!self.comments.contains(&annotation.id) || asked())
    }
}

/// Whether `event` puts another document in front of the app.
pub(crate) const fn opens(event: &Event) -> bool {
    matches!(
        event,
        Event::DocumentOpened(_) | Event::SpaceOpened(_) | Event::CanvasFileChanged { .. }
    )
}

/// Takes the document as it stands as already asked about: it was just
/// opened, and its anchors carry the elements captured when they were
/// placed.
fn seed(app: &mut App) {
    let placed = (app.document.entities())
        .filter_map(|entity| Some((entity.id.clone(), Placed::of(entity)?)))
        .collect();
    let comments = (app.document.annotations().iter())
        .map(|annotation| annotation.id.clone())
        .collect();
    app.session.attach = Attachments {
        placed,
        comments,
        next: app.session.attach.next,
        ..Attachments::default()
    };
    app.session.attach.stamped = true;
}

/// Brings the questions and the pages' tracking in step with the document,
/// once an event has run. `stepped` says the event made, undid or redid a
/// step, which is when an entity can have moved. `opened` says another
/// document is in front, whose anchors already carry their elements.
pub(crate) fn settle(app: &mut App, stepped: bool, opened: bool, effects: &mut Vec<Effect>) {
    if opened {
        seed(app);
    }
    // A page that was just made has been told nothing.
    let mut made = false;
    for effect in effects.iter() {
        if let Effect::CreatePage { page, .. } = effect {
            app.session.attach.tracked.remove(page);
            made = true;
        }
    }
    if stepped {
        ask_moved(app, effects);
    }
    if stepped || made || std::mem::take(&mut app.session.attach.stamped) {
        track(app, effects);
    }
}

/// Asks about every anchored entity that is not where it was last asked
/// about, and every region comment on a page that has not been asked about.
fn ask_moved(app: &mut App, effects: &mut Vec<Effect>) {
    let mut placed = HashMap::new();
    let mut asks: Vec<(Target, EntityId, DVec2)> = Vec::new();
    for entity in app.document.entities() {
        let Some(now) = Placed::of(entity) else {
            continue;
        };
        if app.session.attach.placed.get(&entity.id) != Some(&now)
            && let Some(point) = document_centre(app, &now.page, entity)
        {
            asks.push((Target::Entity(entity.id.clone()), now.page.clone(), point));
        }
        placed.insert(entity.id.clone(), now);
    }
    let mut comments = HashSet::new();
    for annotation in app.document.annotations() {
        if !app.session.attach.comments.contains(&annotation.id)
            && let Some((page, point)) = region_centre(annotation)
        {
            asks.push((Target::Comment(annotation.id.clone()), page, point));
        }
        comments.insert(annotation.id.clone());
    }
    let attach = &mut app.session.attach;
    attach.placed = placed;
    attach.comments = comments;
    for (target, page, point) in asks {
        attach.asked.retain(|_, asked| *asked != target);
        attach.next += 1;
        attach.asked.insert(attach.next, target);
        effects.push(Effect::CaptureElement {
            page,
            request: attach.next,
            point: point.as_vec2(),
        });
    }
}

/// The centre of `entity` as it is seen, in the document CSS pixels of
/// `page`. `None` when the page is gone.
fn document_centre(app: &App, page: &EntityId, entity: &Entity) -> Option<DVec2> {
    let placement = app.page_placement(page)?;
    let seen = scroll_follow::placed_rect(app, entity);
    let centre = geometry::origin(seen) + geometry::size(seen) / 2.0;
    Some(placement.page_local(centre) + app.page_scroll(page))
}

/// The page and the document point a region comment is asked about: the
/// centre of a region stored in a page's document that has no element yet.
fn region_centre(annotation: &Annotation) -> Option<(EntityId, DVec2)> {
    let AnnotationAnchor::Region(RegionAnchor::Document { doc_rect }) = &annotation.anchor else {
        return None;
    };
    let binding = annotation.page_anchor.as_ref()?;
    if binding.element.is_some() {
        return None;
    }
    let centre = geometry::origin(*doc_rect) + geometry::size(*doc_rect) / 2.0;
    Some((binding.page_id.clone(), centre))
}

/// `binding` following `captured`, drawn where it is drawn now: the place
/// recorded for the new element is where it is, less whatever the element
/// before it had travelled, which stays owed.
fn stamped(app: &App, binding: &PageAnchor, captured: &CapturedElement) -> PageAnchor {
    let owed = (app.page_state(&binding.page_id)).map_or(DVec2::ZERO, |state| {
        scroll_follow::element_shift(binding, &state.elements)
    });
    let at = captured.place.doc.as_dvec2() + owed;
    PageAnchor {
        element: Some(AnchorElement {
            selector: captured.selector.clone(),
            doc_x: at.x,
            doc_y: at.y,
            viewport_positioned: captured.place.viewport_positioned.then_some(true),
            extra: JsonMap::new(),
        }),
        ..binding.clone()
    }
}

/// `page` answered the question `request`. The element is stamped on the
/// item it was asked for, with no undo step, unless the item has since been
/// asked about again, is gone, or is on another page now. Returns whether
/// the document changed.
pub(crate) fn on_captured(
    app: &mut App,
    page: &EntityId,
    request: u64,
    captured: Option<&CapturedElement>,
) -> bool {
    let Some(target) = app.session.attach.asked.remove(&request) else {
        return false;
    };
    let Some(captured) = captured else {
        return false;
    };
    let on_page = |binding: &&PageAnchor| binding.page_id == *page;
    let command = match &target {
        Target::Entity(id) => (app.document.entity(id))
            .and_then(|entity| entity.anchor.as_ref())
            .filter(on_page)
            .map(|binding| (binding, stamped(app, binding, captured)))
            .filter(|(binding, next)| *binding != next)
            .map(|(_, next)| Command::SetAnchor {
                id: id.clone(),
                anchor: Some(Box::new(next)),
            }),
        Target::Comment(id) => (app.document.annotations().iter())
            .find(|annotation| annotation.id == *id)
            .and_then(|annotation| Some((annotation, annotation.page_anchor.as_ref()?)))
            .filter(|(_, binding)| on_page(binding))
            .map(|(annotation, binding)| {
                Command::ReplaceAnnotation(Box::new(Annotation {
                    page_anchor: Some(stamped(app, binding, captured)),
                    ..annotation.clone()
                }))
            }),
    };
    let Some(command) = command else {
        return false;
    };
    // The page has just said where the element is.
    (app.session.pages).place(page, &captured.selector, captured.place);
    match app.document.apply(command) {
        Ok(_) => {
            app.session.attach.stamped = true;
            true
        }
        Err(error) => {
            tracing::warn!("element not attached: {error}");
            false
        }
    }
}

/// Tells each page whose set of followed selectors has changed what to
/// track.
fn track(app: &mut App, effects: &mut Vec<Effect>) {
    let mut wanted: HashMap<EntityId, Vec<String>> = HashMap::new();
    let entities = (app.document.entities()).filter_map(|entity| entity.anchor.as_ref());
    let comments = (app.document.annotations().iter())
        .filter_map(|annotation| annotation.page_anchor.as_ref());
    for binding in entities.chain(comments) {
        if let Some(element) = &binding.element {
            let selectors = wanted.entry(binding.page_id.clone()).or_default();
            if !selectors.contains(&element.selector) {
                selectors.push(element.selector.clone());
            }
        }
    }
    let pages: Vec<EntityId> = app.pages().map(|(id, _, _)| id.clone()).collect();
    let tracked = &mut app.session.attach.tracked;
    tracked.retain(|page, _| pages.contains(page));
    for page in pages {
        let mut selectors = wanted.remove(&page).unwrap_or_default();
        selectors.sort_unstable();
        if tracked.get(&page).map_or(&[][..], Vec::as_slice) == selectors {
            continue;
        }
        if selectors.is_empty() {
            tracked.remove(&page);
        } else {
            tracked.insert(page.clone(), selectors.clone());
        }
        effects.push(Effect::TrackElements { page, selectors });
    }
}
