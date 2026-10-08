//! The shell's side of the question the inspect tool asks it, and a reader
//! for what the tool shows.

use specular_core::synthetic::synthetic_inspected_at;
use specular_interact::{Effect, Event, InspectModel, PageNotice};

use crate::TestApp;

impl TestApp {
    /// Answers the latest [`Effect::InspectAt`] not yet drained, as the
    /// shell does: from the synthetic page's grid, laid out at the page's
    /// viewport and not scrolled. A hover and a pick are answered alike.
    #[track_caller]
    pub fn answer_inspect(&mut self) -> &mut Self {
        let asked = self.effects().iter().rev().find_map(|effect| match effect {
            Effect::InspectAt { page, point, pick } => Some((page.clone(), *point, *pick)),
            _ => None,
        });
        let Some((page, point, pick)) = asked else {
            panic!("no page was asked for a node: {:?}", self.effects());
        };
        let Some(placement) = self.app().page_placement(&page) else {
            panic!("page {page:?} is not on the canvas");
        };
        let node = synthetic_inspected_at(placement.viewport, point).map(Box::new);
        self.send(Event::Page {
            page,
            notice: PageNotice::Inspected { point, pick, node },
        })
    }

    /// What the inspect tool draws now: the outline and the popover.
    pub fn inspect_model(&self) -> Option<InspectModel> {
        self.app().inspect()
    }
}
