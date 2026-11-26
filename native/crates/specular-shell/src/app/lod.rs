//! The Electron page-host LOD: each page's texture scale, frame rate and
//! painting follow how large it is on screen and whether it is in view.

use std::time::Instant;

use glam::Vec2;
use specular_bench::PaintPolicy;
use specular_core::{PageId, PageSource};
use specular_doc::EntityId;
use specular_interact::{App, Event, to_canvas_rect};

use super::runtime::{Runtime, ShellWindow};
use crate::paint_lod::{FULL_FRAME_RATE, LodChange};

impl<W: ShellWindow> Runtime<W> {
    /// The window moved to a display of another scale: every page is asked
    /// for frames at it.
    pub fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        self.demand.changed();
        for host in self.hosts.values() {
            let scale = scale_factor as f32 * host.lod.texture().factor();
            if let Err(error) = self.source.set_texture_scale(host.page, scale) {
                tracing::warn!("{error}");
            }
        }
    }

    /// An agent sent each of `pages` something: each stays awake a while,
    /// and one that was not painting starts at once, before the message
    /// reaches it. Nothing drawn changes, so no frame is owed.
    pub(super) fn agent_drove(&mut self, pages: Vec<EntityId>) {
        if pages.is_empty() {
            return;
        }
        for page in pages {
            let effects = specular_interact::update(&mut self.app, Event::AgentDrove(page));
            self.run_all(effects);
        }
        self.settle_pages(Instant::now());
    }

    /// Grades the pages, then sends each one whose viewport or scale changed
    /// this turn its new geometry as one resize: a page shown at a new size
    /// and a new scale is not first painted at the old scale.
    pub(super) fn settle_pages(&mut self, now: Instant) {
        self.pages_settled = true;
        if self.options.paint_policy == PaintPolicy::ElectronLod
            && let Some(viewport) = self.gpu.as_ref().map(W::logical_viewport)
        {
            self.update_paint_lod(viewport, now);
        }
        self.source.flush_geometry();
    }

    /// One layout pass: grades every page by its on-screen scale and
    /// visibility and applies what changed. The page the user is in, entered
    /// or filling its tab, paints at the display's rate. In a window no one
    /// can see, no page paints, except one an agent is driving: it paints
    /// at the display's rate wherever it is.
    fn update_paint_lod(&mut self, viewport: Vec2, now: Instant) {
        let Some(window) = self.gpu.as_ref() else {
            return;
        };
        let window_scale = window.scale_factor();
        let display_rate = window.refresh_rate().unwrap_or(FULL_FRAME_RATE);
        let seen = !window.occluded();
        let camera = self.app.session().camera;
        let prepared = self.app.prepared_page();
        for (id, placement) in self.app.pages() {
            let Some(host) = self.hosts.get_mut(id) else {
                continue;
            };
            // A page prepared for its tab is graded as the tab will show it,
            // so its host gets the tab's viewport and scale as one resize.
            let (display_scale, on_screen) = if prepared == Some(id) {
                (1.0, true)
            } else {
                (
                    placement.display_scale(&camera),
                    camera.is_visible(to_canvas_rect(placement.rect), viewport),
                )
            };
            let (painting, focus_rate) = wake_of(&self.app, id, on_screen && seen, display_rate);
            let change = host.lod.update(display_scale, painting, focus_rate, now);
            apply_lod_change(self.source.as_mut(), host.page, change, window_scale);
        }
    }
}

/// Whether page `id` paints, and the rate it is held at whatever its scale.
/// A page paints while it can be `seen` on screen. The page the user is in
/// (prepared for its tab, entered or filling it) is held at the display's
/// rate, and so is a page an agent is driving, which also paints when no
/// one can see it.
fn wake_of(app: &App, id: &EntityId, seen: bool, display_rate: u32) -> (bool, Option<u32>) {
    let driven = app.agent_driving(id);
    let users = [
        app.prepared_page(),
        app.session().focus.page(),
        app.filling_item(),
    ];
    let in_use = driven || users.contains(&Some(id));
    (seen || driven, in_use.then_some(display_rate))
}

/// Applies `change` in the order Electron's layout pass does: scale before
/// painting, so a page coming into view wakes at the scale it is owed.
fn apply_lod_change(
    source: &mut dyn PageSource,
    page: PageId,
    change: LodChange,
    window_scale: f32,
) {
    if change != LodChange::default() {
        tracing::debug!(%page, ?change, "paint LOD");
    }
    let results = [
        change
            .texture
            .map(|tier| source.set_texture_scale(page, window_scale * tier.factor())),
        change
            .frame_rate
            .map(|fps| source.set_frame_rate(page, fps)),
        change
            .painting
            .map(|painting| source.set_painting(page, painting)),
    ];
    for error in results.into_iter().flatten().filter_map(Result::err) {
        tracing::warn!("{error}");
    }
}

#[cfg(test)]
mod tests {
    use specular_interact::AgentAct;
    use specular_testkit::TestApp;

    use super::*;

    const START_MS: u64 = 1_700_000_000_000;

    fn page(id: &str) -> EntityId {
        EntityId::from(id)
    }

    #[test]
    fn a_page_an_agent_drives_paints_at_the_displays_rate_unseen_until_it_goes_quiet() {
        let mut app = TestApp::with_pages(3);
        app.tick(START_MS);
        app.send(Event::AgentDrove(page("p1")));
        let act = AgentAct {
            ticket: None,
            page: page("p2"),
            session: "agent:p2".to_owned(),
            action: "read".to_owned(),
            at: glam::DVec2::ZERO,
        };
        app.send(Event::AgentAct(act));
        // (page, seen, painting and rate)
        let rows = [
            ("p1", false, (true, Some(120))),
            ("p1", true, (true, Some(120))),
            ("p2", false, (true, Some(120))),
            ("p3", false, (false, None)),
            ("p3", true, (true, None)),
        ];
        for (id, seen, wake) in rows {
            assert_eq!(
                wake_of(app.app(), &page(id), seen, 120),
                wake,
                "{id}, seen {seen}"
            );
        }
        assert_eq!(app.app().agent_wake_lapses_in_ms(), Some(5_000));

        // Driven again, p1 stays awake past p2.
        app.tick(START_MS + 3_000);
        app.send(Event::AgentDrove(page("p1")));
        app.tick(START_MS + 5_000);
        assert_eq!(wake_of(app.app(), &page("p2"), false, 120), (false, None));
        assert_eq!(
            wake_of(app.app(), &page("p1"), false, 120),
            (true, Some(120))
        );
        app.tick(START_MS + 8_000);
        assert_eq!(wake_of(app.app(), &page("p1"), false, 120), (false, None));
        assert_eq!(app.app().agent_wake_lapses_in_ms(), None);
    }
}
