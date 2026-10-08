//! The marks that stand for comments on the canvas: a dashed frame around a
//! region and a count pill for everything else. Hit-testing and the scene
//! both read the set from [`App::comment_marks`], so what is drawn is what
//! can be pressed.

use std::collections::HashMap;

use glam::Vec2;
use specular_doc::{Annotation, AnnotationAnchor, AnnotationId, EntityId};

use super::shown::shown;
use super::{element_on_canvas, region_on_canvas};
use crate::App;
use crate::geometry::ScreenRect;

/// A pill's height, in logical pixels.
pub const PILL_HEIGHT: f32 = 26.0;
/// The width of a pill counting up to nine.
pub const PILL_WIDTH: f32 = 26.0;
/// What each digit after the first adds to a pill's width.
pub const PILL_DIGIT_WIDTH: f32 = 6.0;
/// How far a pill on a page sits inside the page's edge, and how far an
/// element's pill sits inside the corner of the element.
pub const PILL_INSET: f32 = 8.0;
/// How near the top or bottom of its page a pill on a page's side may
/// centre.
pub const PILL_EDGE_MARGIN: f32 = 10.0;
/// How far a region's frame reaches to either side of its edge for a press.
pub const REGION_HIT_BAND: f32 = 6.0;
/// The smallest a region's frame is drawn, each way.
pub const REGION_MIN_SIZE: f32 = 4.0;
/// How far the focus ring stands outside a pill, and how thick it is.
pub const FOCUS_RING_OUTSET: f32 = 3.0;
/// The thickness of the focus ring.
pub const FOCUS_RING_STROKE: f32 = 2.0;

/// What one mark looks like, in logical screen pixels with the origin at the
/// top-left of the canvas view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkShape {
    /// The frame of a region. A press hits its edge, not its inside.
    Region(ScreenRect),
    /// A pill with the number of messages. A press hits all of it.
    Badge(ScreenRect),
}

/// One thing on the canvas that stands for one or more comments.
#[derive(Debug, Clone, PartialEq)]
pub struct CommentMark {
    /// The comment a press focuses: the newest of the members.
    pub annotation: AnnotationId,
    /// Every comment the mark stands for, newest first. Comments on one
    /// element, or one spot of a page, share a pill.
    pub members: Vec<AnnotationId>,
    /// The messages in all the members: each comment and its replies.
    pub count: usize,
    /// Where and what it is.
    pub shape: MarkShape,
    /// Whether the focused comment is one of the members.
    pub focused: bool,
    /// The page the mark is on, when it scrolls with that page: it shows
    /// only through this, and a press outside it misses.
    pub clip: Option<ScreenRect>,
}

impl CommentMark {
    /// Whether a press at `screen` lands on the mark.
    fn hit(&self, screen: Vec2) -> bool {
        if self.clip.is_some_and(|clip| !clip.contains(screen)) {
            return false;
        }
        match self.shape {
            MarkShape::Badge(pill) => pill.contains(screen),
            MarkShape::Region(frame) => {
                frame.inflated(REGION_HIT_BAND).contains(screen)
                    && !frame.inflated(-REGION_HIT_BAND).contains(screen)
            }
        }
    }
}

/// The comment a press at `screen` lands on, frontmost first.
pub(crate) fn at(app: &App, screen: Vec2) -> Option<AnnotationId> {
    (app.comment_marks().into_iter().rev())
        .find(|mark| mark.hit(screen))
        .map(|mark| mark.annotation)
}

impl App {
    /// The marks to draw and to hit, back to front.
    ///
    /// A comment that is resolved, dismissed or about a document its page no
    /// longer shows has none. Comments on a page or an element are grouped
    /// by what they are on, as Electron's badges are; a point or a region
    /// stands alone. The comment being written is not a mark.
    pub fn comment_marks(&self) -> Vec<CommentMark> {
        let mut open: Vec<&Annotation> = (self.document.annotations().iter())
            .filter(|annotation| shown(self, annotation))
            .collect();
        // Timestamps are written in one fixed-width form, so text order is
        // time order. The sort is stable: of two made at once, the earlier
        // in the document is the newer.
        open.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let mut groups: Vec<Vec<&Annotation>> = Vec::new();
        let mut keyed: HashMap<String, usize> = HashMap::new();
        for annotation in open {
            let Some(key) = group_key(annotation) else {
                groups.push(vec![annotation]);
                continue;
            };
            if let Some(&index) = keyed.get(&key) {
                groups[index].push(annotation);
            } else {
                keyed.insert(key, groups.len());
                groups.push(vec![annotation]);
            }
        }
        let focused = self.session.focused_comment.as_ref();
        let mut marks: Vec<CommentMark> = (groups.into_iter())
            .filter_map(|members| {
                let representative = members.first()?;
                let count = members.iter().map(|each| 1 + each.replies.len()).sum();
                Some(CommentMark {
                    annotation: representative.id.clone(),
                    members: members.iter().map(|each| each.id.clone()).collect(),
                    count,
                    shape: shape(self, representative, count)?,
                    focused: focused.is_some_and(|id| members.iter().any(|each| each.id == *id)),
                    clip: super::page_clip(self, representative)
                        .map(|page| ScreenRect::of(&self.session.camera, page)),
                })
            })
            .collect();
        // Newest first above, so the newest ends up in front.
        marks.reverse();
        marks
    }
}

/// What comments on a page or an element are grouped under, or `None` for
/// the anchors that are never grouped.
fn group_key(annotation: &Annotation) -> Option<String> {
    match &annotation.anchor {
        AnnotationAnchor::Page {
            page_id,
            offset_x,
            offset_y,
        } => Some(format!("page:{}:{offset_x}:{offset_y}", page_id.as_str())),
        AnnotationAnchor::Element {
            page_id,
            selector,
            element_path,
            bounding_box,
        } => Some(format!(
            "element:{}:{}:{bounding_box:?}",
            page_id.as_str(),
            element_path.as_deref().unwrap_or(selector),
        )),
        AnnotationAnchor::Canvas { .. } | AnnotationAnchor::Region(_) => None,
    }
}

/// The pill that counts `count` messages.
fn pill_size(count: usize) -> Vec2 {
    let digits = count.checked_ilog10().map_or(1, |log| log as usize + 1);
    Vec2::new(
        PILL_WIDTH + PILL_DIGIT_WIDTH * (digits - 1) as f32,
        PILL_HEIGHT,
    )
}

/// Where `representative`'s mark is, or `None` when what it is on is gone.
fn shape(app: &App, representative: &Annotation, count: usize) -> Option<MarkShape> {
    let camera = &app.session.camera;
    let size = pill_size(count);
    match &representative.anchor {
        AnnotationAnchor::Region(_) => {
            let frame = ScreenRect::of(camera, region_on_canvas(app, representative)?);
            Some(MarkShape::Region(ScreenRect {
                min: frame.min,
                size: frame.size.max(Vec2::splat(REGION_MIN_SIZE)),
            }))
        }
        AnnotationAnchor::Canvas { canvas_x, canvas_y } => {
            let at = camera.world_to_screen(Vec2::new(*canvas_x as f32, *canvas_y as f32));
            Some(MarkShape::Badge(ScreenRect {
                min: at - size / 2.0,
                size,
            }))
        }
        AnnotationAnchor::Page {
            page_id, offset_y, ..
        } => {
            let page = page_on_screen(app, page_id)?;
            // The side of the page, at the height recorded as a fraction of
            // it, kept off the page's top and bottom.
            let middle = (page.min.y + *offset_y as f32 * page.size.y)
                .max(page.min.y + PILL_EDGE_MARGIN)
                .min(page.max().y - PILL_EDGE_MARGIN);
            Some(MarkShape::Badge(ScreenRect {
                min: Vec2::new(page.max().x - PILL_INSET - size.x, middle - size.y / 2.0),
                size,
            }))
        }
        AnnotationAnchor::Element { page_id, .. } => {
            let page = page_on_screen(app, page_id)?;
            // The element's top-right corner, or the page's when the element
            // was recorded with no box, tucked inside the page.
            let element = element_on_canvas(app, representative)
                .map_or(page, |rect| ScreenRect::of(camera, rect));
            let right = (element.max().x - PILL_INSET)
                .min(page.max().x - PILL_INSET)
                .max(page.min.x + PILL_INSET);
            let top = (element.min.y + PILL_INSET)
                .min(page.max().y - PILL_INSET)
                .max(page.min.y + PILL_INSET);
            Some(MarkShape::Badge(ScreenRect {
                min: Vec2::new(right - size.x, top),
                size,
            }))
        }
    }
}

/// The page `id` as the camera shows it.
fn page_on_screen(app: &App, id: &EntityId) -> Option<ScreenRect> {
    let rect = app.page_placement(id)?.rect;
    Some(ScreenRect::of(&app.session.camera, rect))
}
