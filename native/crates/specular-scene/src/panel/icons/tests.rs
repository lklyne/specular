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
        Icon::Proportions,
        Icon::SchemeSystem,
        Icon::SchemeLight,
        Icon::SchemeDark,
        Icon::ChevronDown,
        Icon::Check,
        Icon::Plus,
        Icon::PanelLeft,
        Icon::PanelRight,
        Icon::Eye,
        Icon::EyeOff,
        Icon::Expand,
        Icon::Minimize,
        Icon::Close,
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
