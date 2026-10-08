use specular_doc::ShapeKind;

use super::*;

/// One of every glyph.
fn glyphs() -> Vec<Glyph> {
    let shapes = [
        ShapeKind::Rectangle,
        ShapeKind::Rounded,
        ShapeKind::Ellipse,
        ShapeKind::Diamond,
        ShapeKind::Triangle,
        ShapeKind::Hexagon,
        ShapeKind::Pill,
        ShapeKind::Parallelogram,
        ShapeKind::Chevron,
        ShapeKind::Cylinder,
    ];
    let icons = [
        Icon::SelectTool,
        Icon::PageTool,
        Icon::TextTool,
        Icon::StickyTool,
        Icon::DocumentTool,
        Icon::ShapeTool,
        Icon::DrawPenTool,
        Icon::DrawHighlightTool,
        Icon::CommentTool,
        Icon::AlignLeft,
        Icon::AlignCenter,
        Icon::AlignRight,
        Icon::BrushPen,
        Icon::BrushHighlighter,
        Icon::StrokeThin,
        Icon::StrokeThick,
        Icon::Border,
        Icon::LineSolid,
        Icon::LineDashed,
        Icon::Ban,
        Icon::ArrowStart,
        Icon::ArrowEnd,
        Icon::Trash,
        Icon::Bold,
        Icon::Strikethrough,
        Icon::BulletList,
        Icon::Device,
        Icon::Rotate,
        Icon::SchemeSystem,
        Icon::SchemeLight,
        Icon::SchemeDark,
        Icon::ChevronDown,
        Icon::Check,
        Icon::Plus,
        Icon::PanelLeft,
        Icon::File,
        Icon::FileText,
        Icon::Image,
        Icon::Video,
        Icon::Code,
        Icon::Folder,
        Icon::FolderOpen,
        Icon::StickyNote,
        Icon::PenLine,
        Icon::MessageSquare,
        Icon::Tablet,
        Icon::Laptop,
    ];
    (icons.into_iter())
        .chain(shapes.into_iter().map(Icon::Shape))
        .map(glyph)
        .chain([lucide::CHEVRON, lucide::CHECK])
        .collect()
}

#[test]
fn every_glyph_is_path_data_read_to_its_end() {
    for glyph in glyphs() {
        for layer in glyph.layers {
            if let Shape::Path(d) = layer.shape {
                assert!(svg::reads_all(d), "{d}");
                assert!(!svg::parse(d).is_empty(), "{d}");
            }
        }
    }
}

#[test]
fn a_glyph_is_fitted_into_its_box_keeping_its_proportions() {
    // A 17 by 9 stroke sample in a square: full width, centred down it.
    let mut out = Vec::new();
    let area = Rect::new(100.0, 200.0, 34.0, 34.0);
    draw(Icon::StrokeThin, area, Inks::plain(Color::BLACK), &mut out);
    let [item] = out.as_slice() else {
        panic!("one layer");
    };
    let crate::Draw::Path(path) = &item.draw else {
        panic!("a path");
    };
    // `M0.5 4.42` in the sample's own units, doubled.
    assert_eq!(
        path.commands.first(),
        Some(&PathCommand::MoveTo(Point::new(
            101.0,
            208.0 + 4.42123 * 2.0
        )))
    );
    assert_eq!(path.stroke.map(|stroke| stroke.width), Some(2.0));
}

#[test]
fn a_turned_layer_is_turned_about_its_own_point() {
    // A quarter turn about (10, 10) takes (20, 10) to (10, 20).
    let [xx, yx, xy, yy, dx, dy] = matrix(Turn::Rotate(90.0, 10.0, 10.0));
    let (x, y) = (xx * 20.0 + xy * 10.0 + dx, yx * 20.0 + yy * 10.0 + dy);
    assert!(
        (x - 10.0).abs() < 1e-4 && (y - 20.0).abs() < 1e-4,
        "{x},{y}"
    );
}
