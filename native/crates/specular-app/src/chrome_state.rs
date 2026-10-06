//! What the canvas chrome layer remembers and how pointer gestures change it:
//! the selection, the annotations, the comment tool, and the move, resize and
//! draw-a-region drags.
//!
//! Pure: callers hand in screen positions, the camera and the placed pages,
//! and apply the effects (focus, forwarding, `set_viewport`) themselves.

use glam::Vec2;
use specular_core::{Camera, CanvasRect, CssSize, PageId};

use crate::annotation::Annotation;
use crate::handles::{self, Corner};
use crate::placement::{PlacedPage, hit_test};

/// A comment drag shorter than this many logical pixels is a click and
/// creates nothing.
const MIN_COMMENT_DRAG: f32 = 4.0;

/// An in-progress pointer drag.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Gesture {
    Idle,
    /// Alt-drag of a page; `grab` is the pointer's offset from the rect origin.
    Move {
        page: PageId,
        grab: Vec2,
        start: CanvasRect,
    },
    /// Drag of a selected page's handle; `grab` is the corner's offset from
    /// the pointer so the rect does not jump on press.
    Resize {
        page: PageId,
        corner: Corner,
        grab: Vec2,
        start: CanvasRect,
    },
    /// Drag of the armed comment tool over the canvas.
    Draw {
        start_world: Vec2,
        start_screen: Vec2,
        current_world: Vec2,
        bound: Option<PageId>,
    },
}

/// What a left press did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Press {
    /// A gesture took the press: do not focus or forward it.
    Consumed,
    /// An ordinary click: the selection now follows what was hit, and the
    /// caller focuses and forwards as usual.
    Forward,
}

/// What a left release did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release {
    /// No chrome gesture was running; the caller handles the release.
    NoGesture,
    /// A gesture ended and needs nothing further.
    Finished,
    /// A resize ended: apply `viewport` to the page's source.
    Resized { page: PageId, viewport: CssSize },
}

/// Selection, annotations, tool and gesture state of the chrome layer.
#[derive(Debug)]
pub(crate) struct ChromeState {
    selected: Option<PageId>,
    annotations: Vec<Annotation>,
    tool_armed: bool,
    gesture: Gesture,
}

impl ChromeState {
    /// Chrome showing `annotations`, with nothing selected and the comment
    /// tool off.
    pub(crate) fn new(annotations: Vec<Annotation>) -> Self {
        Self {
            selected: None,
            annotations,
            tool_armed: false,
            gesture: Gesture::Idle,
        }
    }

    pub(crate) fn selected(&self) -> Option<PageId> {
        self.selected
    }

    pub(crate) fn select(&mut self, page: Option<PageId>) {
        self.selected = page;
    }

    pub(crate) fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    pub(crate) fn tool_armed(&self) -> bool {
        self.tool_armed
    }

    /// Whether a drag is running (pointer events stay off the pages).
    pub(crate) fn dragging(&self) -> bool {
        self.gesture != Gesture::Idle
    }

    /// The region the comment tool is drawing, in canvas space.
    pub(crate) fn preview(&self) -> Option<CanvasRect> {
        match self.gesture {
            Gesture::Draw {
                start_world,
                current_world,
                ..
            } => Some(spanning(start_world, current_world)),
            _ => None,
        }
    }

    /// `C`: arms or disarms the comment tool, unless a page has keyboard
    /// focus (then the key belongs to the page) or a drag is running.
    /// Returns whether the key was consumed.
    pub(crate) fn toggle_tool(&mut self, page_focused: bool) -> bool {
        if page_focused || self.dragging() {
            return false;
        }
        self.tool_armed = !self.tool_armed;
        true
    }

    /// `Escape`: abandons the running drag (a move or resize snaps back),
    /// then leaves the comment tool. Clearing page focus is the caller's.
    pub(crate) fn escape(&mut self, placed: &mut [PlacedPage]) {
        if let Gesture::Move { page, start, .. } | Gesture::Resize { page, start, .. } =
            self.gesture
            && let Some(placed) = placed.iter_mut().find(|p| p.page == page)
        {
            placed.rect = start;
        }
        self.gesture = Gesture::Idle;
        self.tool_armed = false;
    }

    /// A left press at `screen`. Priority: the armed comment tool, then a
    /// handle of the selected page, then Alt+press on a page (move), then an
    /// ordinary click that selects the page under it or clears the selection.
    pub(crate) fn press(
        &mut self,
        screen: Vec2,
        camera: &Camera,
        placed: &[PlacedPage],
        alt: bool,
    ) -> Press {
        let world = camera.screen_to_world(screen);
        let under = hit_test(placed, world);
        if self.tool_armed {
            self.gesture = Gesture::Draw {
                start_world: world,
                start_screen: screen,
                current_world: world,
                bound: under.map(|page| page.page),
            };
            return Press::Consumed;
        }
        let selected = self
            .selected
            .and_then(|id| placed.iter().find(|p| p.page == id));
        if let Some(page) = selected
            && let Some(corner) = handles::hit(page.rect, camera, screen)
        {
            self.gesture = Gesture::Resize {
                page: page.page,
                corner,
                grab: corner.point(page.rect) - world,
                start: page.rect,
            };
            return Press::Consumed;
        }
        if alt && let Some(page) = under {
            self.selected = Some(page.page);
            self.gesture = Gesture::Move {
                page: page.page,
                grab: world - page.rect.origin(),
                start: page.rect,
            };
            return Press::Consumed;
        }
        self.selected = under.map(|page| page.page);
        Press::Forward
    }

    /// The pointer moved to `screen` mid-drag: moves or resizes the page
    /// (the viewport is untouched until release) or grows the preview.
    pub(crate) fn drag(&mut self, screen: Vec2, camera: &Camera, placed: &mut [PlacedPage]) {
        let world = camera.screen_to_world(screen);
        match &mut self.gesture {
            Gesture::Idle => {}
            Gesture::Move { page, grab, .. } => {
                if let Some(placed) = placed.iter_mut().find(|p| p.page == *page) {
                    let origin = world - *grab;
                    placed.rect.x = origin.x;
                    placed.rect.y = origin.y;
                }
            }
            Gesture::Resize {
                page,
                corner,
                grab,
                start,
            } => {
                if let Some(placed) = placed.iter_mut().find(|p| p.page == *page) {
                    placed.rect = handles::resized(*start, *corner, world + *grab);
                }
            }
            Gesture::Draw { current_world, .. } => *current_world = world,
        }
    }

    /// The left button came up at `screen`. A comment drag long enough to
    /// not be a click creates its annotation, bound to the page it started
    /// over or to the canvas.
    pub(crate) fn release(
        &mut self,
        screen: Vec2,
        camera: &Camera,
        placed: &[PlacedPage],
    ) -> Release {
        let gesture = std::mem::replace(&mut self.gesture, Gesture::Idle);
        match gesture {
            Gesture::Idle => Release::NoGesture,
            Gesture::Move { .. } => Release::Finished,
            Gesture::Resize { page, .. } => {
                let Some(placed) = placed.iter().find(|p| p.page == page) else {
                    return Release::Finished;
                };
                let size = placed.rect.size().round().max(Vec2::ONE);
                let viewport = CssSize::new(size.x as u32, size.y as u32);
                if viewport == placed.viewport {
                    Release::Finished
                } else {
                    Release::Resized { page, viewport }
                }
            }
            Gesture::Draw {
                start_world,
                start_screen,
                bound,
                ..
            } => {
                if (screen - start_screen).length() >= MIN_COMMENT_DRAG {
                    let region = spanning(start_world, camera.screen_to_world(screen));
                    let owner = bound.and_then(|id| placed.iter().find(|p| p.page == id));
                    self.annotations.push(match owner {
                        Some(page) => Annotation::page_bound(page, region),
                        None => Annotation::canvas_bound(region),
                    });
                }
                Release::Finished
            }
        }
    }
}

/// The normalised rect with `a` and `b` as opposite corners.
fn spanning(a: Vec2, b: Vec2) -> CanvasRect {
    let (low, high) = (a.min(b), a.max(b));
    CanvasRect::new(low.x, low.y, high.x - low.x, high.y - low.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::Anchor;

    const CAMERA: Camera = Camera {
        pan: Vec2::ZERO,
        zoom: 1.0,
    };

    fn pages() -> Vec<PlacedPage> {
        vec![
            PlacedPage::new(
                PageId(1),
                CanvasRect::new(100.0, 100.0, 400.0, 300.0),
                CssSize::new(400, 300),
            ),
            PlacedPage::new(
                PageId(2),
                CanvasRect::new(700.0, 100.0, 400.0, 300.0),
                CssSize::new(400, 300),
            ),
        ]
    }

    fn selecting(page: u64) -> ChromeState {
        let mut state = ChromeState::new(Vec::new());
        state.select(Some(PageId(page)));
        state
    }

    fn armed() -> ChromeState {
        let mut state = ChromeState::new(Vec::new());
        state.toggle_tool(false);
        state
    }

    #[test]
    fn clicking_a_page_selects_it_and_forwards() {
        let mut state = ChromeState::new(Vec::new());
        let press = state.press(Vec2::new(200.0, 200.0), &CAMERA, &pages(), false);
        assert_eq!((press, state.selected()), (Press::Forward, Some(PageId(1))));
    }

    #[test]
    fn clicking_empty_canvas_clears_the_selection() {
        let mut state = selecting(1);
        state.press(Vec2::new(600.0, 600.0), &CAMERA, &pages(), false);
        assert_eq!(state.selected(), None);
    }

    #[test]
    fn alt_drag_moves_the_page_by_the_pointer_delta() {
        let mut state = ChromeState::new(Vec::new());
        let mut placed = pages();
        let press = state.press(Vec2::new(200.0, 150.0), &CAMERA, &placed, true);
        state.drag(Vec2::new(260.0, 130.0), &CAMERA, &mut placed);
        assert_eq!(
            (press, placed[0].rect, state.selected()),
            (
                Press::Consumed,
                CanvasRect::new(160.0, 80.0, 400.0, 300.0),
                Some(PageId(1))
            )
        );
    }

    #[test]
    fn move_is_a_drag_until_release() {
        let mut state = ChromeState::new(Vec::new());
        let mut placed = pages();
        state.press(Vec2::new(200.0, 150.0), &CAMERA, &placed, true);
        state.drag(Vec2::new(210.0, 150.0), &CAMERA, &mut placed);
        let during = state.dragging();
        let release = state.release(Vec2::new(210.0, 150.0), &CAMERA, &placed);
        assert_eq!((during, release), (true, Release::Finished));
    }

    #[test]
    fn move_keeps_the_page_size_and_viewport() {
        let mut state = ChromeState::new(Vec::new());
        let mut placed = pages();
        state.press(Vec2::new(200.0, 150.0), &CAMERA, &placed, true);
        state.drag(Vec2::new(900.0, 900.0), &CAMERA, &mut placed);
        assert_eq!(
            (placed[0].rect.size(), placed[0].viewport),
            (Vec2::new(400.0, 300.0), CssSize::new(400, 300))
        );
    }

    #[test]
    fn move_follows_the_camera_zoom() {
        let camera = Camera::new(Vec2::ZERO, 0.5);
        let mut state = ChromeState::new(Vec::new());
        let mut placed = pages();
        state.press(Vec2::new(100.0, 75.0), &camera, &placed, true);
        state.drag(Vec2::new(110.0, 75.0), &camera, &mut placed);
        assert_eq!(placed[0].rect.x, 120.0);
    }

    #[test]
    fn handle_press_resizes_instead_of_selecting_the_page_under_it() {
        // Page 2 overlaps page 1's bottom-right corner and paints above it.
        let mut placed = pages();
        placed[1].rect = CanvasRect::new(450.0, 350.0, 400.0, 300.0);
        let mut state = selecting(1);
        let press = state.press(Vec2::new(500.0, 400.0), &CAMERA, &placed, false);
        assert_eq!(
            (press, state.selected()),
            (Press::Consumed, Some(PageId(1)))
        );
    }

    #[test]
    fn handles_only_exist_on_the_selected_page() {
        let mut state = ChromeState::new(Vec::new());
        let press = state.press(Vec2::new(500.0, 400.0), &CAMERA, &pages(), false);
        assert_eq!(press, Press::Forward);
    }

    #[test]
    fn resize_drag_changes_the_rect_but_not_the_viewport() {
        let mut state = selecting(1);
        let mut placed = pages();
        state.press(Vec2::new(500.0, 400.0), &CAMERA, &placed, false);
        state.drag(Vec2::new(600.0, 450.0), &CAMERA, &mut placed);
        assert_eq!(
            (placed[0].rect, placed[0].viewport),
            (
                CanvasRect::new(100.0, 100.0, 500.0, 350.0),
                CssSize::new(400, 300)
            )
        );
    }

    #[test]
    fn resize_release_reports_the_viewport_once() {
        let mut state = selecting(1);
        let mut placed = pages();
        state.press(Vec2::new(500.0, 400.0), &CAMERA, &placed, false);
        state.drag(Vec2::new(600.0, 450.0), &CAMERA, &mut placed);
        state.drag(Vec2::new(620.0, 470.0), &CAMERA, &mut placed);
        let first = state.release(Vec2::new(620.0, 470.0), &CAMERA, &placed);
        let second = state.release(Vec2::new(620.0, 470.0), &CAMERA, &placed);
        assert_eq!(
            (first, second),
            (
                Release::Resized {
                    page: PageId(1),
                    viewport: CssSize::new(520, 370)
                },
                Release::NoGesture
            )
        );
    }

    #[test]
    fn resize_from_the_top_left_moves_the_origin() {
        let mut state = selecting(1);
        let mut placed = pages();
        state.press(Vec2::new(102.0, 98.0), &CAMERA, &placed, false);
        state.drag(Vec2::new(52.0, 48.0), &CAMERA, &mut placed);
        assert_eq!(placed[0].rect, CanvasRect::new(50.0, 50.0, 450.0, 350.0));
    }

    #[test]
    fn handle_grab_offset_prevents_a_jump_on_press() {
        let mut state = selecting(1);
        let mut placed = pages();
        state.press(Vec2::new(503.0, 403.0), &CAMERA, &placed, false);
        state.drag(Vec2::new(503.0, 403.0), &CAMERA, &mut placed);
        assert_eq!(placed[0].rect, CanvasRect::new(100.0, 100.0, 400.0, 300.0));
    }

    #[test]
    fn resize_without_a_size_change_reports_nothing() {
        let mut state = selecting(1);
        let placed = pages();
        state.press(Vec2::new(500.0, 400.0), &CAMERA, &placed, false);
        assert_eq!(
            state.release(Vec2::new(500.0, 400.0), &CAMERA, &placed),
            Release::Finished
        );
    }

    #[test]
    fn escape_snaps_a_move_back() {
        let mut state = ChromeState::new(Vec::new());
        let mut placed = pages();
        state.press(Vec2::new(200.0, 150.0), &CAMERA, &placed, true);
        state.drag(Vec2::new(300.0, 300.0), &CAMERA, &mut placed);
        state.escape(&mut placed);
        assert_eq!(
            (placed[0].rect, state.dragging()),
            (CanvasRect::new(100.0, 100.0, 400.0, 300.0), false)
        );
    }

    #[test]
    fn tool_toggles_only_while_no_page_is_focused() {
        let mut state = ChromeState::new(Vec::new());
        let blocked = state.toggle_tool(true);
        let on = state.toggle_tool(false);
        let off = state.toggle_tool(false);
        assert_eq!(
            (blocked, on, off, state.tool_armed()),
            (false, true, true, false)
        );
    }

    #[test]
    fn escape_leaves_the_tool() {
        let mut state = armed();
        state.escape(&mut pages());
        assert!(!state.tool_armed());
    }

    #[test]
    fn armed_tool_draws_instead_of_selecting_or_forwarding() {
        let mut state = armed();
        let press = state.press(Vec2::new(200.0, 200.0), &CAMERA, &pages(), false);
        assert_eq!((press, state.selected()), (Press::Consumed, None));
    }

    #[test]
    fn armed_tool_takes_priority_over_handles_and_alt() {
        let mut state = selecting(1);
        state.toggle_tool(false);
        let press = state.press(Vec2::new(500.0, 400.0), &CAMERA, &pages(), true);
        assert!(state.preview().is_some() && press == Press::Consumed);
    }

    #[test]
    fn preview_is_the_normalised_drag_rect() {
        let mut state = armed();
        let mut placed = pages();
        state.press(Vec2::new(300.0, 300.0), &CAMERA, &placed, false);
        state.drag(Vec2::new(250.0, 340.0), &CAMERA, &mut placed);
        assert_eq!(
            state.preview(),
            Some(CanvasRect::new(250.0, 300.0, 50.0, 40.0))
        );
    }

    #[test]
    fn comment_drag_over_a_page_makes_a_page_bound_annotation() {
        let mut state = armed();
        let mut placed = pages();
        state.press(Vec2::new(200.0, 200.0), &CAMERA, &placed, false);
        state.drag(Vec2::new(300.0, 250.0), &CAMERA, &mut placed);
        state.release(Vec2::new(300.0, 250.0), &CAMERA, &placed);
        let note = state.annotations()[0];
        assert!(matches!(
            note.anchor,
            Anchor::Page {
                page: PageId(1),
                ..
            }
        ));
        assert_eq!(
            note.rect(&placed),
            Some(CanvasRect::new(200.0, 200.0, 100.0, 50.0))
        );
    }

    #[test]
    fn comment_drag_over_empty_canvas_makes_a_canvas_bound_annotation() {
        let mut state = armed();
        let mut placed = pages();
        state.press(Vec2::new(550.0, 500.0), &CAMERA, &placed, false);
        state.drag(Vec2::new(650.0, 560.0), &CAMERA, &mut placed);
        state.release(Vec2::new(650.0, 560.0), &CAMERA, &placed);
        assert!(matches!(state.annotations()[0].anchor, Anchor::Canvas(_)));
    }

    #[test]
    fn a_click_with_the_tool_creates_nothing() {
        let mut state = armed();
        let placed = pages();
        state.press(Vec2::new(200.0, 200.0), &CAMERA, &placed, false);
        state.release(Vec2::new(201.0, 201.0), &CAMERA, &placed);
        assert_eq!(state.annotations().len(), 0);
    }

    #[test]
    fn the_tool_stays_armed_after_a_drag() {
        let mut state = armed();
        let placed = pages();
        state.press(Vec2::new(200.0, 200.0), &CAMERA, &placed, false);
        state.release(Vec2::new(300.0, 300.0), &CAMERA, &placed);
        assert!(state.tool_armed());
    }

    #[test]
    fn escape_mid_draw_cancels_without_creating() {
        let mut state = armed();
        let mut placed = pages();
        state.press(Vec2::new(200.0, 200.0), &CAMERA, &placed, false);
        state.drag(Vec2::new(300.0, 300.0), &CAMERA, &mut placed);
        state.escape(&mut placed);
        state.release(Vec2::new(300.0, 300.0), &CAMERA, &placed);
        assert_eq!(state.annotations().len(), 0);
    }
}
