//! The questions the app has put to pages and not had answered: which page
//! and point an element answer is for, and which region the counts from
//! several pages add up to.
//!
//! A page answers through its event queue, some frames after it was asked,
//! and the app's own answer events repeat the question. This holds the
//! question in between.

use std::collections::HashMap;

use glam::Vec2;
use specular_core::{CssRect, InspectedNode, PageElement};
use specular_doc::{EntityId, Rect};
use specular_interact::{Event, PageGrab, PageNotice};

/// A rect in a page's CSS pixels as the page source takes it.
pub(crate) fn css_rect(rect: Rect) -> CssRect {
    CssRect::new(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    )
}

/// A region whose pages have not all said what it grabbed in them.
#[derive(Debug)]
struct Grab {
    region: Rect,
    /// The pages in the order asked, each with its count once known.
    pages: Vec<(EntityId, Option<usize>)>,
}

impl Grab {
    fn answer(&self) -> Option<Event> {
        let grabs: Option<Vec<PageGrab>> = (self.pages.iter())
            .map(|(page, elements)| {
                Some(PageGrab {
                    page: page.clone(),
                    elements: (*elements)?,
                })
            })
            .collect();
        Some(Event::RegionGrab {
            region: self.region,
            grabs: grabs?,
        })
    }
}

/// The open questions, by the request number each was asked under.
#[derive(Debug, Default)]
pub(crate) struct PageQueries {
    next: u64,
    elements: HashMap<u64, (EntityId, Vec2)>,
    /// The page, the point and whether a click asked, for each inspect
    /// question.
    inspects: HashMap<u64, (EntityId, Vec2, bool)>,
    /// Which grab, and which of its pages, a request is for.
    grab_requests: HashMap<u64, (u64, usize)>,
    grabs: HashMap<u64, Grab>,
    /// The page each question about a region still being dragged is for.
    previews: HashMap<u64, EntityId>,
}

impl PageQueries {
    fn request(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// Whether every question has been answered.
    pub(crate) fn is_idle(&self) -> bool {
        self.elements.is_empty()
            && self.inspects.is_empty()
            && self.grabs.is_empty()
            && self.previews.is_empty()
    }

    /// Notes that `page` is being asked what a region still being dragged
    /// has inside it, and returns the request number to ask under.
    pub(crate) fn ask_preview(&mut self, page: EntityId) -> u64 {
        let request = self.request();
        self.previews.insert(request, page);
        request
    }

    /// The event that answers a question about a region still being
    /// dragged, or `None` for a request that is not one.
    pub(crate) fn preview_answer(&mut self, request: u64, boxes: &[CssRect]) -> Option<Event> {
        let page = self.previews.remove(&request)?;
        Some(Event::Page {
            page,
            notice: PageNotice::RegionElements(boxes.to_vec()),
        })
    }

    /// Notes that `page` is being asked for the element at `point`, and
    /// returns the request number to ask under.
    pub(crate) fn ask_element(&mut self, page: EntityId, point: Vec2) -> u64 {
        let request = self.request();
        self.elements.insert(request, (page, point));
        request
    }

    /// The event that answers an element question, or `None` for a request
    /// that is not open.
    pub(crate) fn element_answer(
        &mut self,
        request: u64,
        element: Option<PageElement>,
    ) -> Option<Event> {
        let (page, point) = self.elements.remove(&request)?;
        Some(Event::ElementAt {
            page,
            point,
            element,
        })
    }

    /// Notes that `page` is being asked for the node at `point` as the
    /// inspect tool reads it, and returns the request number to ask under.
    pub(crate) fn ask_inspect(&mut self, page: EntityId, point: Vec2, pick: bool) -> u64 {
        let request = self.request();
        self.inspects.insert(request, (page, point, pick));
        request
    }

    /// The event that answers an inspect question, or `None` for a request
    /// that is not open.
    pub(crate) fn inspect_answer(
        &mut self,
        request: u64,
        node: Option<Box<InspectedNode>>,
    ) -> Option<Event> {
        let (page, point, pick) = self.inspects.remove(&request)?;
        Some(Event::Page {
            page,
            notice: PageNotice::Inspected { point, pick, node },
        })
    }

    /// Notes that `pages` are being asked what `region` grabbed in each, and
    /// returns the request number to ask each under, in order. With no
    /// pages there is nothing to wait for: [`settled`](Self::settled) has
    /// the answer at once.
    pub(crate) fn ask_grab(&mut self, region: Rect, pages: Vec<EntityId>) -> Vec<u64> {
        let grab = self.request();
        let requests: Vec<u64> = (0..pages.len())
            .map(|index| {
                let request = self.request();
                self.grab_requests.insert(request, (grab, index));
                request
            })
            .collect();
        let pages = pages.into_iter().map(|page| (page, None)).collect();
        self.grabs.insert(grab, Grab { region, pages });
        requests
    }

    /// Records one page's count for the region asked under `request`.
    pub(crate) fn grab_answer(&mut self, request: u64, elements: usize) {
        if let Some((grab, index)) = self.grab_requests.remove(&request)
            && let Some(grab) = self.grabs.get_mut(&grab)
            && let Some((_, count)) = grab.pages.get_mut(index)
        {
            *count = Some(elements);
        }
    }

    /// The answer events for the regions every page has now answered for,
    /// oldest first. Each is returned once.
    pub(crate) fn settled(&mut self) -> Vec<Event> {
        let mut done: Vec<u64> = (self.grabs.iter())
            .filter(|(_, grab)| grab.pages.iter().all(|(_, count)| count.is_some()))
            .map(|(id, _)| *id)
            .collect();
        done.sort_unstable();
        done.into_iter()
            .filter_map(|id| self.grabs.remove(&id)?.answer())
            .collect()
    }

    /// Answers everything still open for `page`, which will not answer for
    /// itself: it closed, crashed or refused. It has no element and grabbed
    /// nothing. Returns the element answers; region answers come from
    /// [`settled`](Self::settled).
    pub(crate) fn give_up_on(&mut self, page: &EntityId) -> Vec<Event> {
        let mut requests: Vec<u64> = (self.elements.iter())
            .filter(|(_, (asked, _))| asked == page)
            .map(|(request, _)| *request)
            .collect();
        requests.sort_unstable();
        let answers = requests
            .into_iter()
            .filter_map(|request| self.element_answer(request, None))
            .collect();
        // A hover or a pick of a page that is gone has nothing to show.
        self.inspects.retain(|_, (asked, _, _)| asked != page);
        self.previews.retain(|_, asked| asked != page);
        let grabs = &self.grabs;
        let waiting: Vec<u64> = (self.grab_requests.iter())
            .filter(|(_, (grab, index))| {
                (grabs.get(grab).and_then(|grab| grab.pages.get(*index)))
                    .is_some_and(|(asked, _)| asked == page)
            })
            .map(|(request, _)| *request)
            .collect();
        for request in waiting {
            self.grab_answer(request, 0);
        }
        answers
    }
}
