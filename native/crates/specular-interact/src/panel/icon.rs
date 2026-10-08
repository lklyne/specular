//! [`Icon`]: every glyph the toolbar and item popup models use, by name.
//!
//! A renderer maps each name to path data. Each variant says where the
//! Electron app's glyph lives (`src/renderer/...`), so the shape can be
//! lifted from there. Lucide names are from `lucide-react`.

use specular_doc::ShapeKind;

/// A glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// The select tool. `shared/icons/toolbar/select.svg`.
    SelectTool,
    /// The page tool. `AddPageToolIcon` in `shared/CustomIcons.tsx`.
    PageTool,
    /// The text tool. `shared/icons/toolbar/add-text.svg`.
    TextTool,
    /// The sticky tool. `AddStickyToolIcon` in `shared/CustomIcons.tsx`,
    /// drawn in the sticky color.
    StickyTool,
    /// The document tool. `shared/icons/toolbar/add-document.svg`.
    DocumentTool,
    /// The shape tool. `AddShapeToolIcon` in `shared/CustomIcons.tsx`,
    /// drawn in the shape color.
    ShapeTool,
    /// The draw tool with the pen brush. `DrawPenToolIcon` in
    /// `shared/CustomIcons.tsx`, drawn in the stroke color.
    DrawPenTool,
    /// The draw tool with the highlighter. `DrawHighlightToolIcon` in
    /// `shared/CustomIcons.tsx`, drawn in the stroke color.
    DrawHighlightTool,
    /// The comment tool. `CommentToolIcon` in `shared/CustomIcons.tsx`.
    CommentTool,
    /// A shape silhouette: the path of that kind's row in
    /// `src/shared/shapes.ts`, drawn by `shared/ShapeGlyph.tsx`.
    Shape(ShapeKind),
    /// Left-aligned text. Lucide `AlignLeft`.
    AlignLeft,
    /// Centered text. Lucide `AlignCenter`.
    AlignCenter,
    /// Right-aligned text. Lucide `AlignRight`.
    AlignRight,
    /// The pen brush. `PenSlimIcon` in `shared/CustomIcons.tsx`, drawn in
    /// the stroke color.
    BrushPen,
    /// The highlighter brush. `PenMarkerIcon` in `shared/CustomIcons.tsx`,
    /// drawn in the stroke color.
    BrushHighlighter,
    /// A thin stroke. `StrokeThinIcon` in `shared/CustomIcons.tsx`.
    StrokeThin,
    /// A thick stroke. `StrokeThickIcon` in `shared/CustomIcons.tsx`.
    StrokeThick,
    /// A border: three stacked lines. `BorderGlyph` in
    /// `above-view/BorderDropdown.tsx` (Figma node 527:63).
    Border,
    /// A solid line. `LineGlyph` in `above-view/BorderDropdown.tsx`.
    LineSolid,
    /// A dashed line. `LineGlyph` in `above-view/EdgeStrokeDropdown.tsx`.
    LineDashed,
    /// Nothing painted. Lucide `Ban`.
    Ban,
    /// An arrowhead at an edge's start. Lucide `ArrowLeft`.
    ArrowStart,
    /// An arrowhead at an edge's end. Lucide `ArrowRight`.
    ArrowEnd,
    /// Delete. Lucide `Trash2`.
    Trash,
    /// Bold. Lucide `Bold`.
    Bold,
    /// Strikethrough. Lucide `Strikethrough`.
    Strikethrough,
    /// A bullet list. Lucide `List`.
    BulletList,
    /// A device frame. Lucide `Smartphone`.
    Device,
    /// Rotate the viewport. `RotateIcon` in `shared/CustomIcons.tsx`.
    Rotate,
    /// Back in a page's history. Lucide `ChevronLeft`.
    ChevronLeft,
    /// Forward in a page's history. Lucide `ChevronRight`.
    ChevronRight,
    /// Load a page again. Lucide `RotateCw`.
    Reload,
    /// Abandon a page's load. Lucide `X`.
    Stop,
    /// The system color scheme. `shared/icons/toolbar/sun-moon.svg`.
    SchemeSystem,
    /// The light color scheme. `shared/icons/toolbar/sun.svg`.
    SchemeLight,
    /// The dark color scheme. `shared/icons/toolbar/moon.svg`.
    SchemeDark,
}
