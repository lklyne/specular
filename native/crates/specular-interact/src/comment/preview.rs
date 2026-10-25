//! What the comment tool marks on pages before it comments: the element
//! under the pointer, and the elements a region being dragged has inside it.
//!
//! The hover is the inspect tool's question and answer
//! (`inspect::hover`). The region is asked of each page under the drag as
//! it changes ([`Effect::PreviewRegion`]), and the boxes a page answers
//! with are outlined until the button comes up.

use specular_core::CssRect;
use specular_doc::{EntityId, Rect};

use super::grab::{self, PageRegion};
use crate::{App, Effect, ScreenRect, Tool};

/// The elements under a comment region being dragged.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RegionPreview {
    /// The pages last asked, each with the part of it the region covered,
    /// so a move that covers the same CSS pixels asks nothing.
    asked: Vec<PageRegion>,
    /// The boxes each page answered with, in its viewport CSS pixels.
    boxes: Vec<(EntityId, Vec<CssRect>)>,
}

/// The pointer moved with a comment region in flight: asks the pages under
/// it what it has inside it now.
pub(crate) fn follow(app: &mut App, effects: &mut Vec<Effect>) {
    let Some(region) = app.session.comment_preview() else {
        return;
    };
    let pages: Vec<PageRegion> = (grab::pages_under(app, region).into_iter())
        .map(|covered| PageRegion {
            rect: Rect::new(
                covered.rect.x.round(),
                covered.rect.y.round(),
                covered.rect.width.round(),
                covered.rect.height.round(),
            ),
            ..covered
        })
        .collect();
    let preview = &mut app.session.region_preview;
    if preview.asked == pages {
        return;
    }
    (preview.boxes).retain(|(page, _)| pages.iter().any(|covered| covered.page == *page));
    preview.asked.clone_from(&pages);
    if !pages.is_empty() {
        effects.push(Effect::PreviewRegion { pages });
    }
}

/// `page` said what the region has inside it. An answer that finds no
/// region in flight, or the page no longer under it, is dropped.
pub(crate) fn on_elements(app: &mut App, page: &EntityId, boxes: &[CssRect]) {
    if app.session.comment_preview().is_none() {
        return;
    }
    let preview = &mut app.session.region_preview;
    if !preview.asked.iter().any(|covered| covered.page == *page) {
        return;
    }
    preview.boxes.retain(|(held, _)| held != page);
    preview.boxes.push((page.clone(), boxes.to_vec()));
}

/// Forgets the region's elements once no region is being dragged.
pub(crate) fn settle(app: &mut App) {
    if app.session.comment_preview().is_none() {
        app.session.region_preview = RegionPreview::default();
    }
}

impl App {
    /// The page elements the comment tool marks, in screen pixels: the ones
    /// inside the region being dragged, or with no drag the one under the
    /// pointer. Each is drawn as the inspect tool's outline.
    pub fn comment_highlights(&self) -> Vec<ScreenRect> {
        if self.session.tool != Tool::Comment {
            return Vec::new();
        }
        let on_screen = |page: &EntityId, css: Rect| {
            let placement = self.page_placement(page)?;
            let canvas = super::css_on_canvas(placement, css);
            Some(crate::geometry::on_screen(&self.session.camera, canvas))
        };
        if self.session.comment_preview().is_some() {
            return (self.session.region_preview.boxes.iter())
                .flat_map(|(page, boxes)| boxes.iter().map(move |css| (page, css)))
                .filter_map(|(page, css)| {
                    let css = Rect::new(
                        f64::from(css.x),
                        f64::from(css.y),
                        f64::from(css.width),
                        f64::from(css.height),
                    );
                    on_screen(page, css)
                })
                .collect();
        }
        if self.session.gesture.is_some() {
            return Vec::new();
        }
        (self.session.inspect.hovered.iter())
            .filter_map(|target| {
                let css = target.node.bounding_box;
                let css = Rect::new(
                    f64::from(css.x),
                    f64::from(css.y),
                    f64::from(css.width),
                    f64::from(css.height),
                );
                on_screen(&target.page, css)
            })
            .collect()
    }
}
