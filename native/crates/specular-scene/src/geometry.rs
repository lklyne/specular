//! Points, sizes and rects. They carry no unit: an [`Item`](crate::Item)'s
//! [`Space`](crate::Space) says whether they are canvas units or logical
//! screen pixels.

/// A position.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Horizontal position, growing rightwards.
    pub x: f32,
    /// Vertical position, growing downwards.
    pub y: f32,
}

impl Point {
    /// Creates a point.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A width and height.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Size {
    /// Creates a size.
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rect.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Rect {
    /// Creates a rect.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Top-left corner.
    pub const fn origin(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Width and height.
    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    /// The middle of the rect.
    pub fn centre(self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Right edge.
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge.
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// The rect grown by `by` on every side (shrunk when negative, never
    /// below zero size).
    #[must_use]
    pub fn outset(self, by: f32) -> Self {
        let width = (self.width + by * 2.0).max(0.0);
        let height = (self.height + by * 2.0).max(0.0);
        let centre = self.centre();
        Self::new(
            centre.x - width * 0.5,
            centre.y - height * 0.5,
            width,
            height,
        )
    }

    /// Whether the two rects overlap with positive area.
    pub fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// The overlap of the two rects, or `None` when it has no area.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (right > x && bottom > y).then(|| Self::new(x, y, right - x, bottom - y))
    }

    /// The smallest rect holding both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Self::new(x, y, right - x, bottom - y)
    }

    /// The smallest rect holding every point, or `None` for no points.
    pub fn bounding(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        let (mut min, mut max) = (first, first);
        for point in points {
            min = Point::new(min.x.min(point.x), min.y.min(point.y));
            max = Point::new(max.x.max(point.x), max.y.max(point.y));
        }
        Some(Self::new(min.x, min.y, max.x - min.x, max.y - min.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rects_touching_at_an_edge_do_not_intersect() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 0.0, 10.0, 10.0);
        assert_eq!((a.intersects(b), a.intersection(b)), (false, None));
    }

    #[test]
    fn intersection_is_the_shared_area() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(6.0, -2.0, 10.0, 5.0);
        assert_eq!(a.intersection(b), Some(Rect::new(6.0, 0.0, 4.0, 3.0)));
    }

    #[test]
    fn outset_grows_every_side_and_stops_at_zero() {
        let rect = Rect::new(10.0, 10.0, 4.0, 8.0);
        assert_eq!(
            (rect.outset(1.0), rect.outset(-3.0)),
            (
                Rect::new(9.0, 9.0, 6.0, 10.0),
                Rect::new(12.0, 13.0, 0.0, 2.0)
            )
        );
    }

    #[test]
    fn bounding_spans_all_points() {
        let points = [
            Point::new(3.0, 9.0),
            Point::new(-1.0, 4.0),
            Point::new(2.0, 12.0),
        ];
        assert_eq!(Rect::bounding(points), Some(Rect::new(-1.0, 4.0, 4.0, 8.0)));
    }
}
