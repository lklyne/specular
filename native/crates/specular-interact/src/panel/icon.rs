//! [`Icon`]: every glyph the toolbar and item dock models use, by name.
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
    /// The inspect tool. `shared/icons/toolbar/inspect.svg`.
    InspectTool,
    /// A page's repo binding. Lucide `FolderCode`.
    Repo,
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
    /// Arrange in a row. Lucide `Columns2`: side-by-side bars read as a row.
    ArrangeRow,
    /// Arrange in a column. Lucide `Rows2`.
    ArrangeColumn,
    /// Arrange in a grid. Lucide `Grid2x2`.
    ArrangeGrid,
    /// Annotate. Lucide `MessageCircle`.
    Annotate,
    /// A device frame. Lucide `Smartphone`.
    Device,
    /// Rotate the viewport. `RotateIcon` in `shared/CustomIcons.tsx`.
    Rotate,
    /// A sync set. Lucide `Link2`.
    Sync,
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
    /// The down chevron of an open section. Lucide `ChevronDown`.
    ChevronDown,
    /// The mark beside the active canvas. Lucide `Check`.
    Check,
    /// Add a canvas. Lucide `Plus`.
    Plus,
    /// The sidebar's toggle. Lucide `PanelRight`, mirrored.
    PanelLeft,
    /// The right panel's toggle. Lucide `PanelRight`.
    PanelRight,
    /// The eye, open: an item view draws what is around its item. Lucide
    /// `Eye`.
    Eye,
    /// The eye, shut. Lucide `EyeOff`.
    EyeOff,
    /// Open the item in a tab of its own. Lucide `Maximize2`.
    Expand,
    /// Close a tab. Lucide `X`.
    Close,
    /// A canvas, and a file of no known kind. Lucide `File`.
    File,
    /// A markdown document. Lucide `FileText`.
    FileText,
    /// An image file. Lucide `Image`.
    Image,
    /// A video file. Lucide `Video`.
    Video,
    /// A web document. Lucide `Code`.
    Code,
    /// A closed group. Lucide `Folder`.
    Folder,
    /// An open group. Lucide `FolderOpen`.
    FolderOpen,
    /// Plain text and sticky notes. Lucide `StickyNote`.
    StickyNote,
    /// A freehand drawing. Lucide `PenLine`.
    PenLine,
    /// A comment. Lucide `MessageSquare`.
    MessageSquare,
    /// A tablet-sized page. Lucide `Tablet`.
    Tablet,
    /// A desktop-sized page. Lucide `Laptop`.
    Laptop,
}
