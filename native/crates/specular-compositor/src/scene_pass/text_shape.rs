//! Shaping a [`TextRun`] into a glyphon buffer: its style, the spans set
//! differently, and the underlines and strikes those spans ask for.

use std::ops::Range;

use glyphon::cosmic_text::{
    Align, Ellipsize, EllipsizeHeightLimit, TextDecoration, UnderlineStyle,
};
use glyphon::{
    Attrs, Buffer, Color as GlyphColor, Family, FontSystem, Metrics, Shaping, Style, Weight, Wrap,
};
use specular_scene::{Color, FontFamily, Rect, Size, SpanStyle, TextAlign, TextOverflow, TextRun};

use super::emoji;

/// The thinnest an underline or strike is drawn, as a fraction of the font
/// size, for a font that reports no thickness.
const LEAST_LINE_EM: f32 = 1.0 / 16.0;

/// An underline or a strike, in the buffer's units from its top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Line {
    pub(crate) rect: Rect,
    /// The span's colour; `None` takes the run's.
    pub(crate) color: Option<Color>,
}

/// A run shaped and wrapped.
pub(crate) struct ShapedText {
    pub(crate) buffer: Buffer,
    /// The size of the lines.
    pub(crate) size: Size,
    pub(crate) lines: Vec<Line>,
}

/// Shapes and wraps `run`; `None` when its metrics cannot be shaped.
pub(crate) fn shape(fonts: &mut FontSystem, run: &TextRun) -> Option<ShapedText> {
    let usable = |value: f32| value.is_finite() && value > 0.0;
    if !usable(run.size) || !usable(run.line_height) {
        return None;
    }
    let attrs = Attrs::new()
        .family(family(&run.family))
        .weight(Weight(run.weight))
        .style(style(run.italic));
    let align = match run.align {
        // `None` lets right-to-left text start from the right.
        TextAlign::Left => None,
        TextAlign::Centre => Some(Align::Center),
        TextAlign::Right => Some(Align::Right),
    };
    let mut buffer = Buffer::new_empty(Metrics::new(run.size, run.line_height));
    match (run.wrap_width, run.overflow) {
        (None, _) => buffer.set_wrap(Wrap::None),
        (Some(_), TextOverflow::Wrap) => buffer.set_wrap(Wrap::WordOrGlyph),
        (Some(_), TextOverflow::Ellipsis) => {
            buffer.set_wrap(Wrap::None);
            buffer.set_ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)));
        }
    }
    buffer.set_size(run.wrap_width, None);
    let emoji = emoji::ranges(&run.text);
    if run.spans.is_empty() && emoji.is_empty() {
        buffer.set_text(&run.text, &attrs, Shaping::Advanced, align);
    } else {
        let pieces = with_emoji(run, pieces(run, &attrs), &emoji);
        buffer.set_rich_text(pieces, &attrs, Shaping::Advanced, align);
    }
    buffer.shape_until_scroll(fonts, false);
    let mut size = measure(&buffer);
    if run.wrap_width.is_none() && align.is_some() {
        // Lines align inside a width, so an unwrapped run is given the width
        // of its longest line and laid out again.
        buffer.set_size(Some(size.width), None);
        buffer.shape_until_scroll(fonts, false);
        size = Size::new(size.width, measure(&buffer).height);
    }
    let lines = lines(&buffer);
    Some(ShapedText {
        buffer,
        size,
        lines,
    })
}

fn family(family: &FontFamily) -> Family<'_> {
    match family {
        FontFamily::SansSerif => Family::SansSerif,
        FontFamily::Serif => Family::Serif,
        FontFamily::Monospace => Family::Monospace,
        FontFamily::Named(name) => Family::Name(name),
    }
}

fn style(italic: bool) -> Style {
    if italic { Style::Italic } else { Style::Normal }
}

/// The run's text cut at its spans, each piece with the attributes it is set
/// in. A span that is out of order, out of range or off a character boundary
/// is skipped, and its text keeps the run's style.
fn pieces<'a>(run: &'a TextRun, base: &Attrs<'a>) -> Vec<(&'a str, Attrs<'a>)> {
    let text = run.text.as_str();
    if run.spans.is_empty() {
        return vec![(text, base.clone())];
    }
    let mut pieces = Vec::with_capacity(run.spans.len() * 2 + 1);
    let mut at = 0;
    for span in &run.spans {
        let (start, end) = (span.range.start, span.range.end);
        let usable = start >= at
            && start < end
            && end <= text.len()
            && text.is_char_boundary(start)
            && text.is_char_boundary(end);
        if !usable {
            continue;
        }
        if start > at {
            pieces.push((&text[at..start], base.clone()));
        }
        pieces.push((&text[start..end], span_attrs(base, &span.style)));
        at = end;
    }
    if at < text.len() {
        pieces.push((&text[at..], base.clone()));
    }
    pieces
}

/// `pieces` cut again at each emoji range, with the emoji set at their own
/// size. They keep the line height, so a line with one is no taller.
fn with_emoji<'a>(
    run: &TextRun,
    pieces: Vec<(&'a str, Attrs<'a>)>,
    emoji: &[Range<usize>],
) -> Vec<(&'a str, Attrs<'a>)> {
    if emoji.is_empty() {
        return pieces;
    }
    let set = emoji::setting(run.size);
    let mut out = Vec::with_capacity(pieces.len() + emoji.len() * 2);
    let mut start = 0;
    for (piece, attrs) in pieces {
        let end = start + piece.len();
        let mut at = start;
        for range in emoji {
            let (from, to) = (range.start.max(at), range.end.min(end));
            if from >= to {
                continue;
            }
            if from > at {
                out.push((&piece[at - start..from - start], attrs.clone()));
            }
            let coloured = attrs
                .clone()
                .family(Family::Name(emoji::FAMILY))
                .metrics(Metrics::new(set.size, run.line_height))
                .letter_spacing(set.spacing);
            out.push((&piece[from - start..to - start], coloured));
            at = to;
        }
        if at < end {
            out.push((&piece[at - start..], attrs));
        }
        start = end;
    }
    out
}

fn span_attrs<'a>(base: &Attrs<'a>, span: &'a SpanStyle) -> Attrs<'a> {
    let mut attrs = base.clone();
    if let Some(named) = &span.family {
        attrs.family = family(named);
    }
    if let Some(weight) = span.weight {
        attrs.weight = Weight(weight);
    }
    if let Some(italic) = span.italic {
        attrs.style = style(italic);
    }
    if let Some(color) = span.color {
        attrs.color_opt = Some(GlyphColor::rgba(color.r, color.g, color.b, color.a));
    }
    attrs.text_decoration = TextDecoration {
        underline: if span.underline {
            UnderlineStyle::Single
        } else {
            UnderlineStyle::None
        },
        strikethrough: span.strike,
        ..TextDecoration::new()
    };
    attrs
}

fn measure(buffer: &Buffer) -> Size {
    buffer
        .layout_runs()
        .fold(Size::default(), |size, line| Size {
            width: size.width.max(line.line_w),
            height: size.height.max(line.line_top + line.line_height),
        })
}

/// The underlines and strikes of a shaped buffer. glyphon draws glyphs only,
/// so these are laid out here from the font's own offsets and thicknesses.
fn lines(buffer: &Buffer) -> Vec<Line> {
    let mut lines = Vec::new();
    for run in buffer.layout_runs() {
        for span in run.decorations {
            let glyphs = &run.glyphs[span.glyph_range.clone()];
            // Not first and last: right-to-left text stores them reversed.
            let (left, right) = glyphs
                .iter()
                .fold((f32::MAX, f32::MIN), |(left, right), g| {
                    (left.min(g.x), right.max(g.x + g.w))
                });
            if right <= left {
                continue;
            }
            let color = span
                .color_opt
                .map(|color| Color::rgba(color.r(), color.g(), color.b(), color.a()));
            let decoration = &span.data.text_decoration;
            let mut line = |metrics: glyphon::cosmic_text::DecorationMetrics| {
                let thickness = metrics.thickness.max(LEAST_LINE_EM) * span.font_size;
                let top = run.line_y - metrics.offset * span.font_size;
                lines.push(Line {
                    rect: Rect::new(left, top, right - left, thickness),
                    color,
                });
            };
            if decoration.underline != UnderlineStyle::None {
                line(span.data.underline_metrics);
            }
            if decoration.strikethrough {
                line(span.data.strikethrough_metrics);
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use specular_scene::Point;

    use super::*;

    fn title(text: &str, width: f32) -> TextRun {
        TextRun {
            wrap_width: Some(width),
            overflow: TextOverflow::Ellipsis,
            ..TextRun::new(text, Point::default(), 11.0, Color::BLACK)
        }
    }

    fn shaped(run: &TextRun) -> (Size, String) {
        let mut fonts = FontSystem::new();
        let shaped = shape(&mut fonts, run);
        let glyphs = shaped.as_ref().map_or_else(String::new, |shaped| {
            (shaped.buffer.layout_runs())
                .flat_map(|line| line.glyphs.iter())
                .map(|glyph| &line_text(&shaped.buffer)[glyph.start..glyph.end])
                .collect()
        });
        (
            shaped.map_or_else(Size::default, |shaped| shaped.size),
            glyphs,
        )
    }

    fn line_text(buffer: &Buffer) -> &str {
        buffer.lines.first().map_or("", |line| line.text())
    }

    #[test]
    fn an_ellipsised_run_is_one_line_inside_its_width() {
        let long = "A long title for a small group";
        let (size, _) = shaped(&title(long, 80.0));
        assert!(size.width <= 80.0, "{size:?}");
        assert!((size.height - 11.0 * TextRun::DEFAULT_LINE_HEIGHT).abs() < 0.01);
        // Wrapping at the same width takes more than one line.
        let wrapped = TextRun {
            overflow: TextOverflow::Wrap,
            ..title(long, 80.0)
        };
        assert!(shaped(&wrapped).0.height > size.height * 1.5);
    }
}
