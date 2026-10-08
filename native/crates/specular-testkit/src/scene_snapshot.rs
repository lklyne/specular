//! A [`Scene`] as text for `insta`.

use std::fmt::Write as _;

use specular_scene::{
    Blend, Color, ColumnDraw, Dash, Draw, FontFamily, Item, PathCommand, PathStroke, Point, Rect,
    RuleHeight, Scene, Space, SpanStyle, Stroke, StrokeAlign, TextAlign, TextOverflow, TextRun,
    VerticalAlign,
};

/// A path longer than this is summarised by its length and bounds. A
/// freehand outline has dozens of points and none of them is worth reading.
const LONGEST_SPELLED_PATH: usize = 12;

/// The scene as stable text: one line per item in paint order, back to
/// front. Numbers are rounded to a hundredth, colours are CSS hex. A column
/// is followed by a line for each row, cell and rule, indented under it.
///
/// ```text
/// canvas page p1 100,100 400x300 r=8
/// screen rect 100,100 400x300 r=8 stroke=#a6a09b/1/outside
/// screen text "example.com/p1" 100,79.5 box-h=16.5 11/15.4 w500 #6b6b6b middle clip=100,79.5 400x20.5
/// ```
pub fn scene_snapshot(scene: &Scene) -> String {
    let lines: Vec<String> = scene.items.iter().map(item_line).collect();
    lines.join("\n")
}

fn item_line(item: &Item) -> String {
    let mut out = String::from(match item.space {
        Space::Canvas => "canvas ",
        Space::Screen => "screen ",
    });
    // Writing to a `String` cannot fail.
    let _ = write_draw(&mut out, &item.draw);
    if let Some(clip) = item.clip {
        let _ = write!(out, " clip={}", rect(clip));
    }
    if item.opacity < 1.0 {
        let _ = write!(out, " opacity={}", num(item.opacity));
    }
    match item.blend {
        Blend::Normal => {}
        Blend::Multiply => out.push_str(" blend=multiply"),
    }
    out
}

fn write_draw(out: &mut String, draw: &Draw) -> std::fmt::Result {
    match draw {
        Draw::Page(page) => {
            write!(out, "page {} {}", page.page, rect(page.rect))?;
            write_radius(out, page.corner_radius)
        }
        Draw::Shadow(shadow) => {
            write!(out, "shadow {}", rect(shadow.rect))?;
            write_radius(out, shadow.corner_radius)?;
            write!(out, " blur={}", num(shadow.blur))?;
            write_fill(out, Some(shadow.color))
        }
        Draw::Rect(shape) => {
            write!(out, "rect {}", rect(shape.rect))?;
            write_radius(out, shape.corner_radius)?;
            write_fill(out, shape.fill)?;
            write_stroke(out, shape.stroke)
        }
        Draw::Ellipse(ellipse) => {
            write!(out, "ellipse {}", rect(ellipse.rect))?;
            write_fill(out, ellipse.fill)?;
            write_stroke(out, ellipse.stroke)
        }
        Draw::Polygon(polygon) => {
            let points: Vec<String> = polygon.points.iter().map(|at| point(*at)).collect();
            write!(out, "polygon {}", points.join(" "))?;
            write_fill(out, polygon.fill)?;
            write_path_stroke(out, polygon.stroke)
        }
        Draw::Path(path) => {
            write!(out, "path ")?;
            if path.commands.len() > LONGEST_SPELLED_PATH {
                write!(out, "{} commands", path.commands.len())?;
                if let Some(bounds) = draw_bounds(&path.commands) {
                    write!(out, " in {}", rect(bounds))?;
                }
            } else {
                let commands: Vec<String> = path.commands.iter().map(command).collect();
                write!(out, "{}", commands.join(" "))?;
            }
            write_fill(out, path.fill)?;
            write_path_stroke(out, path.stroke)
        }
        Draw::Text(run) => write_text(out, run),
        Draw::Column(column) => write_column(out, column),
        Draw::Image(image) => {
            write!(out, "image {} {}", image.image.0, rect(image.rect))?;
            if image.source != specular_scene::ImageDraw::WHOLE {
                write!(out, " source={}", rect(image.source))?;
            }
            write_radius(out, image.corner_radius)
        }
    }
}

fn write_text(out: &mut String, run: &TextRun) -> std::fmt::Result {
    write!(out, "text {:?} {}", run.text, point(run.origin))?;
    if let Some(width) = run.wrap_width {
        write!(out, " wrap={}", num(width))?;
        match run.overflow {
            TextOverflow::Wrap => {}
            TextOverflow::Ellipsis => write!(out, " ellipsis")?,
        }
    }
    if let Some(height) = run.box_height {
        write!(out, " box-h={}", num(height))?;
    }
    write!(out, " {}/{}", num(run.size), num(run.line_height))?;
    write_family(out, Some(&run.family))?;
    if run.weight != 400 {
        write!(out, " w{}", run.weight)?;
    }
    if run.italic {
        write!(out, " italic")?;
    }
    write!(out, " {}", color(run.color))?;
    match run.align {
        TextAlign::Left => {}
        TextAlign::Centre => write!(out, " centre")?,
        TextAlign::Right => write!(out, " right")?,
    }
    match run.vertical_align {
        VerticalAlign::Top => {}
        VerticalAlign::Middle => write!(out, " middle")?,
        VerticalAlign::Bottom => write!(out, " bottom")?,
    }
    if run.spans.is_empty() {
        return Ok(());
    }
    let spans: Vec<String> = (run.spans.iter())
        .map(|span| {
            let range = &span.range;
            format!("{}..{}{}", range.start, range.end, span_style(&span.style))
        })
        .collect();
    write!(out, " spans=[{}]", spans.join(", "))
}

/// What a span changes, each part led by a space.
fn span_style(style: &SpanStyle) -> String {
    let mut out = String::new();
    // Writing to a `String` cannot fail.
    let _ = write_family(&mut out, style.family.as_ref());
    if let Some(weight) = style.weight {
        let _ = write!(out, " w{weight}");
    }
    match style.italic {
        Some(true) => out.push_str(" italic"),
        Some(false) => out.push_str(" upright"),
        None => {}
    }
    if let Some(ink) = style.color {
        let _ = write!(out, " {}", color(ink));
    }
    if style.underline {
        out.push_str(" underline");
    }
    if style.strike {
        out.push_str(" strike");
    }
    out
}

fn write_family(out: &mut String, family: Option<&FontFamily>) -> std::fmt::Result {
    match family {
        Some(FontFamily::SansSerif) | None => Ok(()),
        Some(FontFamily::Serif) => write!(out, " serif"),
        Some(FontFamily::Monospace) => write!(out, " mono"),
        Some(FontFamily::Named(name)) => write!(out, " {name:?}"),
    }
}

fn write_column(out: &mut String, column: &ColumnDraw) -> std::fmt::Result {
    write!(
        out,
        "column {} {}x{}",
        point(column.origin),
        num(column.width),
        num(column.height)
    )?;
    if column.scroll != 0.0 {
        write!(out, " scroll={}", num(column.scroll))?;
    }
    for row in &column.rows {
        write!(out, "\n  row")?;
        if row.gap != 0.0 {
            write!(out, " gap={}", num(row.gap))?;
        }
        if row.min_height != 0.0 {
            write!(out, " min-h={}", num(row.min_height))?;
        }
        if row.bottom_padding != 0.0 {
            write!(out, " pad-b={}", num(row.bottom_padding))?;
        }
        for rule in &row.rules {
            write!(out, "\n    rule x={} w={}", num(rule.x), num(rule.width))?;
            match rule.height {
                RuleHeight::Row => write!(out, " row")?,
                RuleHeight::RowAndGap => write!(out, " row+gap")?,
                RuleHeight::Middle(height) => write!(out, " middle={}", num(height))?,
                RuleHeight::Bottom(height) => write!(out, " bottom={}", num(height))?,
            }
            write!(out, " {}", color(rule.color))?;
        }
        for cell in &row.cells {
            write!(out, "\n    ")?;
            write_text(out, cell)?;
        }
    }
    Ok(())
}

fn write_radius(out: &mut String, radius: f32) -> std::fmt::Result {
    if radius == 0.0 {
        return Ok(());
    }
    write!(out, " r={}", num(radius))
}

fn write_fill(out: &mut String, fill: Option<Color>) -> std::fmt::Result {
    fill.map_or(Ok(()), |fill| write!(out, " fill={}", color(fill)))
}

fn write_stroke(out: &mut String, stroke: Option<Stroke>) -> std::fmt::Result {
    let Some(stroke) = stroke else {
        return Ok(());
    };
    let align = match stroke.align {
        StrokeAlign::Inside => "inside",
        StrokeAlign::Centre => "centre",
        StrokeAlign::Outside => "outside",
    };
    write!(
        out,
        " stroke={}/{}/{align}",
        color(stroke.color),
        num(stroke.width)
    )
}

fn write_path_stroke(out: &mut String, stroke: Option<PathStroke>) -> std::fmt::Result {
    let Some(stroke) = stroke else {
        return Ok(());
    };
    write!(out, " stroke={}/{}", color(stroke.color), num(stroke.width))?;
    // Round caps and joins are the default and go unsaid.
    let default = PathStroke::new(stroke.color, stroke.width);
    if stroke.cap != default.cap {
        write!(out, " cap={:?}", stroke.cap)?;
    }
    if stroke.join != default.join {
        write!(out, " join={:?}", stroke.join)?;
    }
    if let Some(Dash { on, off }) = stroke.dash {
        write!(out, " dash={}/{}", num(on), num(off))?;
    }
    Ok(())
}

fn command(command: &PathCommand) -> String {
    match *command {
        PathCommand::MoveTo(to) => format!("M{}", point(to)),
        PathCommand::LineTo(to) => format!("L{}", point(to)),
        PathCommand::QuadTo { control, to } => format!("Q{} {}", point(control), point(to)),
        PathCommand::CubicTo {
            control1,
            control2,
            to,
        } => format!("C{} {} {}", point(control1), point(control2), point(to)),
        PathCommand::Close => "Z".to_owned(),
    }
}

/// The box around every point a path names.
fn draw_bounds(commands: &[PathCommand]) -> Option<Rect> {
    let points = commands.iter().flat_map(|command| match *command {
        PathCommand::MoveTo(to) | PathCommand::LineTo(to) => vec![to],
        PathCommand::QuadTo { control, to } => vec![control, to],
        PathCommand::CubicTo {
            control1,
            control2,
            to,
        } => vec![control1, control2, to],
        PathCommand::Close => Vec::new(),
    });
    Rect::bounding(points)
}

/// `value` to a hundredth, without trailing zeros.
fn num(value: f32) -> String {
    let rounded = (f64::from(value) * 100.0).round() / 100.0;
    // No negative zero in a snapshot.
    format!("{}", if rounded == 0.0 { 0.0 } else { rounded })
}

fn point(at: Point) -> String {
    format!("{},{}", num(at.x), num(at.y))
}

fn rect(rect: Rect) -> String {
    format!(
        "{},{} {}x{}",
        num(rect.x),
        num(rect.y),
        num(rect.width),
        num(rect.height)
    )
}

/// `#rrggbb`, or `#rrggbbaa` when not opaque.
fn color(color: Color) -> String {
    let Color { r, g, b, a } = color;
    if a == u8::MAX {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

/// Asserts that the scene a [`TestApp`](crate::TestApp) shows matches its
/// snapshot. Takes what [`assert_doc_snapshot!`](crate::assert_doc_snapshot)
/// takes:
///
/// ```ignore
/// assert_scene_snapshot!(app);                    // tests/snapshots/<test name>.snap
/// assert_scene_snapshot!(app, @"canvas page …");  // inline
/// assert_scene_snapshot!("while_dragging", app);
/// ```
#[macro_export]
macro_rules! assert_scene_snapshot {
    ($name:literal, $app:expr $(,)?) => {
        $crate::insta::assert_snapshot!($name, $app.scene_snapshot())
    };
    ($app:expr $(, $($rest:tt)*)?) => {
        $crate::insta::assert_snapshot!($app.scene_snapshot() $(, $($rest)*)?)
    };
}
