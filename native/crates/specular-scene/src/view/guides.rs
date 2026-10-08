//! The guides a move or a resize shows: a line where an edge or a centre
//! sits on a neighbour's, and a capped measure in each gap of a run that is
//! evenly spaced. All in screen space, so the lines keep their weight at
//! any zoom.

use glam::DVec2;
use specular_interact::{DistributionGap, GuideAxis};

use super::frame::Frame;
use crate::{Color, Item, PathDraw, PathStroke, Point, Scene};

const ALIGNMENT_WIDTH: f32 = 1.0;
const DISTRIBUTION_COLOR: Color = Color::rgb(0xec, 0x48, 0x99);
const DISTRIBUTION_WIDTH: f32 = 1.5;
/// How far a measure's end caps reach either side of it.
const CAP_HALF: f32 = 9.0;
/// How far a measure stops short of the rects it sits between.
const CAP_INSET: f32 = 1.0;

pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    let guides = frame.app.guides();
    let stroke = PathStroke::new(frame.colors.selection, ALIGNMENT_WIDTH);
    for guide in &guides.alignment {
        let (from, to) = match guide.axis {
            GuideAxis::Horizontal => (
                DVec2::new(guide.start, guide.coordinate),
                DVec2::new(guide.end, guide.coordinate),
            ),
            GuideAxis::Vertical => (
                DVec2::new(guide.coordinate, guide.start),
                DVec2::new(guide.coordinate, guide.end),
            ),
        };
        let line = [frame.screen_point(from), frame.screen_point(to)];
        scene.push(Item::screen(PathDraw::polyline(line, stroke)));
    }
    for guide in &guides.distribution {
        for gap in &guide.gaps {
            scene.extend(measure(frame, guide.axis, *gap));
        }
    }
}

/// One gap's measure: a line across it with a cap at each end.
fn measure(frame: &Frame<'_>, axis: GuideAxis, gap: DistributionGap) -> [Item; 3] {
    let stroke = PathStroke::new(DISTRIBUTION_COLOR, DISTRIBUTION_WIDTH);
    let line = |from: Point, to: Point| Item::screen(PathDraw::polyline([from, to], stroke));
    match axis {
        GuideAxis::Horizontal => {
            let from = frame.screen_point(DVec2::new(gap.start, gap.cross));
            let to = frame.screen_point(DVec2::new(gap.end, gap.cross));
            let (y, x0, x1) = (from.y, from.x + CAP_INSET, to.x - CAP_INSET);
            [
                line(Point::new(x0, y), Point::new(x1, y)),
                line(Point::new(x0, y - CAP_HALF), Point::new(x0, y + CAP_HALF)),
                line(Point::new(x1, y - CAP_HALF), Point::new(x1, y + CAP_HALF)),
            ]
        }
        GuideAxis::Vertical => {
            let from = frame.screen_point(DVec2::new(gap.cross, gap.start));
            let to = frame.screen_point(DVec2::new(gap.cross, gap.end));
            let (x, y0, y1) = (from.x, from.y + CAP_INSET, to.y - CAP_INSET);
            [
                line(Point::new(x, y0), Point::new(x, y1)),
                line(Point::new(x - CAP_HALF, y0), Point::new(x + CAP_HALF, y0)),
                line(Point::new(x - CAP_HALF, y1), Point::new(x + CAP_HALF, y1)),
            ]
        }
    }
}
