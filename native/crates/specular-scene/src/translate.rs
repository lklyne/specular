//! Moving a finished item, for drawing something where it is not: the
//! copies an Option-drag is about to leave.

use crate::{Draw, Item, PathCommand, Point, Rect};

impl Item {
    /// The same item moved by `(dx, dy)` in its own space, clip included.
    #[must_use]
    pub fn translated(self, dx: f32, dy: f32) -> Self {
        Self {
            clip: self.clip.map(|clip| moved(clip, dx, dy)),
            draw: self.draw.translated(dx, dy),
            ..self
        }
    }
}

impl Draw {
    /// The same draw moved by `(dx, dy)`. A column that reported its height
    /// for an entity no longer does: the entity's own column still will.
    #[must_use]
    pub fn translated(self, dx: f32, dy: f32) -> Self {
        let at = |point: Point| Point::new(point.x + dx, point.y + dy);
        match self {
            Self::Page(mut page) => {
                page.rect = moved(page.rect, dx, dy);
                Self::Page(page)
            }
            Self::Shadow(mut shadow) => {
                shadow.rect = moved(shadow.rect, dx, dy);
                Self::Shadow(shadow)
            }
            Self::Rect(mut rect) => {
                rect.rect = moved(rect.rect, dx, dy);
                Self::Rect(rect)
            }
            Self::Ellipse(mut ellipse) => {
                ellipse.rect = moved(ellipse.rect, dx, dy);
                Self::Ellipse(ellipse)
            }
            Self::Polygon(mut polygon) => {
                for point in &mut polygon.points {
                    *point = at(*point);
                }
                Self::Polygon(polygon)
            }
            Self::Path(mut path) => {
                for command in &mut path.commands {
                    *command = match *command {
                        PathCommand::MoveTo(to) => PathCommand::MoveTo(at(to)),
                        PathCommand::LineTo(to) => PathCommand::LineTo(at(to)),
                        PathCommand::QuadTo { control, to } => PathCommand::QuadTo {
                            control: at(control),
                            to: at(to),
                        },
                        PathCommand::CubicTo {
                            control1,
                            control2,
                            to,
                        } => PathCommand::CubicTo {
                            control1: at(control1),
                            control2: at(control2),
                            to: at(to),
                        },
                        PathCommand::Close => PathCommand::Close,
                    };
                }
                Self::Path(path)
            }
            Self::Text(mut run) => {
                run.origin = at(run.origin);
                Self::Text(run)
            }
            Self::Column(mut column) => {
                column.origin = at(column.origin);
                column.owner = None;
                Self::Column(column)
            }
            Self::Image(mut image) => {
                image.rect = moved(image.rect, dx, dy);
                Self::Image(image)
            }
        }
    }
}

fn moved(rect: Rect, dx: f32, dy: f32) -> Rect {
    Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RectDraw};

    #[test]
    fn a_moved_item_keeps_its_size_and_takes_its_clip_along() {
        let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
        let item = Item::canvas(RectDraw::filled(rect, Color::WHITE)).clipped(rect);
        let moved = item.translated(5.0, -5.0);
        let there = Rect::new(15.0, 15.0, 30.0, 40.0);
        assert_eq!(
            (moved.draw.bounds(), moved.clip),
            (Some(there), Some(there))
        );
    }
}
