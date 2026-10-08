//! SVG path data as scene path commands.
//!
//! The glyphs are kept as the `d` strings the Electron app ships, so a
//! changed icon is a pasted string. Every command letter is read: lines,
//! both Béziers with their smooth forms, and elliptical arcs, which become
//! cubics. Data that stops making sense ends the path where it stops.

use std::f32::consts::{FRAC_PI_2, TAU};

use crate::{PathCommand, Point};

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn skip_separators(&mut self) {
        while (self.bytes.get(self.at))
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b',')
        {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_separators();
        self.bytes.get(self.at).copied()
    }

    /// One number. A sign or a second decimal point starts the next one, so
    /// `1-2` and `.5.5` are two numbers each.
    fn number(&mut self) -> Option<f32> {
        self.skip_separators();
        let start = self.at;
        let mut seen_point = false;
        let mut seen_exponent = false;
        while let Some(&byte) = self.bytes.get(self.at) {
            let after_exponent =
                self.at > start && matches!(self.bytes.get(self.at - 1), Some(b'e' | b'E'));
            let fits = match byte {
                b'0'..=b'9' => true,
                b'+' | b'-' => self.at == start || after_exponent,
                b'.' if !seen_point && !seen_exponent => {
                    seen_point = true;
                    true
                }
                b'e' | b'E' if !seen_exponent && self.at > start => {
                    seen_exponent = true;
                    true
                }
                _ => false,
            };
            if !fits {
                break;
            }
            self.at += 1;
        }
        std::str::from_utf8(self.bytes.get(start..self.at)?)
            .ok()?
            .parse()
            .ok()
    }

    /// An arc flag: one digit, which may run straight into the next number.
    fn flag(&mut self) -> Option<bool> {
        let flag = match self.peek()? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.at += 1;
        Some(flag)
    }

    fn point(&mut self) -> Option<Point> {
        Some(Point::new(self.number()?, self.number()?))
    }
}

/// Where the pen is while a path is read.
#[derive(Default)]
struct Pen {
    at: Point,
    /// Where the open sub-path began, which `Z` returns to.
    start: Point,
    /// The last control point of the curve just drawn, for the smooth forms
    /// to mirror: a cubic's for `S`, a quadratic's for `T`.
    cubic: Option<Point>,
    quad: Option<Point>,
    out: Vec<PathCommand>,
}

fn add(a: Point, b: Point) -> Point {
    Point::new(a.x + b.x, a.y + b.y)
}

/// `control` mirrored through `at`, or `at` itself with nothing to mirror.
fn mirrored(at: Point, control: Option<Point>) -> Point {
    control.map_or(at, |control| {
        Point::new(2.0 * at.x - control.x, 2.0 * at.y - control.y)
    })
}

impl Pen {
    /// `point` as an absolute position.
    fn resolve(&self, point: Point, relative: bool) -> Point {
        if relative { add(self.at, point) } else { point }
    }

    fn line_to(&mut self, to: Point) {
        self.out.push(PathCommand::LineTo(to));
        (self.at, self.cubic, self.quad) = (to, None, None);
    }

    fn cubic_to(&mut self, control1: Point, control2: Point, to: Point) {
        self.out.push(PathCommand::CubicTo {
            control1,
            control2,
            to,
        });
        (self.at, self.cubic, self.quad) = (to, Some(control2), None);
    }

    fn quad_to(&mut self, control: Point, to: Point) {
        self.out.push(PathCommand::QuadTo { control, to });
        (self.at, self.cubic, self.quad) = (to, None, Some(control));
    }

    /// Reads the arguments of one `command` and draws it. `None` when they
    /// are missing or the letter is not a command.
    fn step(&mut self, command: u8, reader: &mut Reader<'_>) -> Option<()> {
        let relative = command.is_ascii_lowercase();
        match command.to_ascii_uppercase() {
            b'M' => {
                let to = self.resolve(reader.point()?, relative);
                self.out.push(PathCommand::MoveTo(to));
                (self.at, self.start, self.cubic, self.quad) = (to, to, None, None);
            }
            b'L' => {
                let to = self.resolve(reader.point()?, relative);
                self.line_to(to);
            }
            b'H' => {
                let x = reader.number()?;
                let x = if relative { self.at.x + x } else { x };
                self.line_to(Point::new(x, self.at.y));
            }
            b'V' => {
                let y = reader.number()?;
                let y = if relative { self.at.y + y } else { y };
                self.line_to(Point::new(self.at.x, y));
            }
            b'C' => {
                let (c1, c2, to) = (reader.point()?, reader.point()?, reader.point()?);
                self.cubic_to(
                    self.resolve(c1, relative),
                    self.resolve(c2, relative),
                    self.resolve(to, relative),
                );
            }
            b'S' => {
                let (c2, to) = (reader.point()?, reader.point()?);
                let c1 = mirrored(self.at, self.cubic);
                self.cubic_to(c1, self.resolve(c2, relative), self.resolve(to, relative));
            }
            b'Q' => {
                let (control, to) = (reader.point()?, reader.point()?);
                self.quad_to(self.resolve(control, relative), self.resolve(to, relative));
            }
            b'T' => {
                let to = reader.point()?;
                let control = mirrored(self.at, self.quad);
                self.quad_to(control, self.resolve(to, relative));
            }
            b'A' => {
                let radii = reader.point()?;
                let turn = reader.number()?;
                let (large, sweep) = (reader.flag()?, reader.flag()?);
                let to = self.resolve(reader.point()?, relative);
                arc(self, radii, turn.to_radians(), large, sweep, to);
            }
            b'Z' => {
                self.out.push(PathCommand::Close);
                (self.at, self.cubic, self.quad) = (self.start, None, None);
            }
            _ => return None,
        }
        Some(())
    }
}

/// An elliptical arc from the pen to `to` as cubics, one per quarter turn
/// or less, by the SVG specification's endpoint-to-centre conversion.
fn arc(pen: &mut Pen, radii: Point, turn: f32, large: bool, sweep: bool, to: Point) {
    let from = pen.at;
    let (mut rx, mut ry) = (radii.x.abs(), radii.y.abs());
    if rx <= f32::EPSILON || ry <= f32::EPSILON || from == to {
        pen.line_to(to);
        return;
    }
    let (sin, cos) = turn.sin_cos();
    let half = Point::new((from.x - to.x) / 2.0, (from.y - to.y) / 2.0);
    let x1 = cos * half.x + sin * half.y;
    let y1 = -sin * half.x + cos * half.y;
    // Radii too small to span the two points are grown until they do.
    let reach = (x1 / rx).powi(2) + (y1 / ry).powi(2);
    if reach > 1.0 {
        rx *= reach.sqrt();
        ry *= reach.sqrt();
    }
    let spread = (rx * y1).powi(2) + (ry * x1).powi(2);
    let room = ((rx * ry).powi(2) - spread).max(0.0);
    let side = if large == sweep { -1.0 } else { 1.0 };
    let scale = side * (room / spread).sqrt();
    let centre = Point::new(scale * rx * y1 / ry, -scale * ry * x1 / rx);
    let angle_of = |x: f32, y: f32| y.atan2(x);
    let first = angle_of((x1 - centre.x) / rx, (y1 - centre.y) / ry);
    let mut swept = angle_of((-x1 - centre.x) / rx, (-y1 - centre.y) / ry) - first;
    if sweep && swept < 0.0 {
        swept += TAU;
    } else if !sweep && swept > 0.0 {
        swept -= TAU;
    }
    let middle = Point::new(f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y));
    // A point on the unit circle, set on the ellipse.
    let place = |x: f32, y: f32| {
        let (x, y) = (centre.x + rx * x, centre.y + ry * y);
        Point::new(cos * x - sin * y + middle.x, sin * x + cos * y + middle.y)
    };
    // A hair under, so a sweep of exactly a half turn is two pieces, not
    // three, whatever the rounding.
    let pieces = (swept.abs() / FRAC_PI_2 - 1e-4).ceil().max(1.0);
    let step = swept / pieces;
    let handle = 4.0 / 3.0 * (step / 4.0).tan();
    for piece in 0..pieces as usize {
        let (a, b) = (
            first + step * piece as f32,
            first + step * (piece + 1) as f32,
        );
        let (sin_a, cos_a) = a.sin_cos();
        let (sin_b, cos_b) = b.sin_cos();
        let end = if piece + 1 == pieces as usize {
            to
        } else {
            place(cos_b, sin_b)
        };
        pen.cubic_to(
            place(cos_a - handle * sin_a, sin_a + handle * cos_a),
            place(cos_b + handle * sin_b, sin_b - handle * cos_b),
            end,
        );
    }
}

/// The commands of the SVG path data `d`.
pub(crate) fn parse(d: &str) -> Vec<PathCommand> {
    read(d).0
}

/// The commands of `d`, and whether all of it was read.
fn read(d: &str) -> (Vec<PathCommand>, bool) {
    let mut reader = Reader {
        bytes: d.as_bytes(),
        at: 0,
    };
    let mut pen = Pen::default();
    let mut last = None;
    while let Some(byte) = reader.peek() {
        let command = if byte.is_ascii_alphabetic() {
            reader.at += 1;
            byte
        } else {
            // Numbers with no letter repeat the command before them, and
            // after a move they are lines.
            match last {
                Some(b'M') => b'L',
                Some(b'm') => b'l',
                Some(b'Z' | b'z') | None => return (pen.out, false),
                Some(command) => command,
            }
        };
        if pen.step(command, &mut reader).is_none() {
            return (pen.out, false);
        }
        last = Some(command);
    }
    (pen.out, true)
}

/// Whether every byte of `d` is path data this module reads.
#[cfg(test)]
pub(super) fn reads_all(d: &str) -> bool {
    read(d).1
}

#[cfg(test)]
mod tests;
