//! The toolbar, and the popup of each tool that has one. A tool's popup
//! reads and writes the tool defaults.

use specular_doc::BrushType;
use specular_interact::{Action, Tool, ToolDefaultPatch};
use specular_testkit::{TestApp, assert_popup_snapshot, assert_toolbar_snapshot, shape};

#[test]
fn the_toolbar_at_rest_has_the_eight_tools_in_groups_and_the_zoom_levels() {
    let app = TestApp::empty();
    assert_toolbar_snapshot!(app, @r#"
    tool [x] tool.select "Select" icon=SelectTool chord=v -> SetTool(Select)
    ---
    tool [ ] tool.draw "Draw" icon=DrawPenTool color=1 chord=m -> SetTool(Draw)
    tool [ ] tool.sticky "Sticky note" icon=StickyTool color=3 chord=s -> SetTool(AddSticky)
    tool [ ] tool.shape "Shape" icon=ShapeTool color=1 chord=r -> SetTool(AddShape)
    tool [ ] tool.page "Page" icon=PageTool chord=p -> SetTool(AddPage)
    tool [ ] tool.text "Text" icon=TextTool chord=t -> SetTool(AddText)
    tool [ ] tool.document "Document" icon=DocumentTool -> SetTool(AddDocument)
    ---
    tool [ ] tool.comment "Comment" icon=CommentTool chord=c -> SetTool(Comment)
    ---
    dropdown zoom "Zoom" shows text="100%"
      options list
        option [ ] zoom.10 "Zoom to 10%" text="10%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 0.1 })
        option [ ] zoom.25 "Zoom to 25%" text="25%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 0.25 })
        option [ ] zoom.50 "Zoom to 50%" text="50%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 0.5 })
        option [ ] zoom.75 "Zoom to 75%" text="75%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 0.75 })
        option [x] zoom.100 "Zoom to 100%" text="100%" chord=cmd+0 -> ZoomReset
        option [ ] zoom.150 "Zoom to 150%" text="150%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 1.5 })
        option [ ] zoom.200 "Zoom to 200%" text="200%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 2.0 })
    "#);
}

#[test]
fn the_toolbar_follows_the_active_tool_the_defaults_and_the_zoom() {
    let mut app = TestApp::empty();
    app.act(Action::SetToolVariant(ToolDefaultPatch::Brush(
        BrushType::Highlight,
    )));
    app.act(Action::SetToolDefault(ToolDefaultPatch::DrawColor(
        specular_doc::Color::Preset(specular_doc::ColorPreset::Green),
    )));
    app.act(Action::ZoomOut);
    assert_toolbar_snapshot!(app, @r#"
    tool [ ] tool.select "Select" icon=SelectTool chord=v -> SetTool(Select)
    ---
    tool [x] tool.draw "Draw" icon=DrawHighlightTool color=4 chord=m -> SetTool(Select)
    tool [ ] tool.sticky "Sticky note" icon=StickyTool color=3 chord=s -> SetTool(AddSticky)
    tool [ ] tool.shape "Shape" icon=ShapeTool color=1 chord=r -> SetTool(AddShape)
    tool [ ] tool.page "Page" icon=PageTool chord=p -> SetTool(AddPage)
    tool [ ] tool.text "Text" icon=TextTool chord=t -> SetTool(AddText)
    tool [ ] tool.document "Document" icon=DocumentTool -> SetTool(AddDocument)
    ---
    tool [ ] tool.comment "Comment" icon=CommentTool chord=c -> SetTool(Comment)
    ---
    dropdown zoom "Zoom" shows text="80%"
      options list
        option [ ] zoom.10 "Zoom to 10%" text="10%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 0.1 })
        option [ ] zoom.25 "Zoom to 25%" text="25%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 0.25 })
        option [ ] zoom.50 "Zoom to 50%" text="50%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 0.5 })
        option [ ] zoom.75 "Zoom to 75%" text="75%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 0.75 })
        option [ ] zoom.100 "Zoom to 100%" text="100%" chord=cmd+0 -> ZoomReset
        option [ ] zoom.150 "Zoom to 150%" text="150%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 1.5 })
        option [ ] zoom.200 "Zoom to 200%" text="200%" -> SetCamera(Camera { pan: Vec2(0.0, 0.0), zoom: 2.0 })
    "#);
}

#[test]
fn the_text_tool_popup_sets_the_plain_text_defaults() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddText);
    assert_popup_snapshot!(app, @r#"
    anchor toolbar gap=8
    dropdown text.size "Set default text size" shows text="Small"
      options list
        option [x] text.size.14 "Small" -> SetToolDefault(TextSize(14.0))
        option [ ] text.size.32 "Medium" -> SetToolDefault(TextSize(32.0))
        option [ ] text.size.56 "Large" -> SetToolDefault(TextSize(56.0))
        option [ ] text.size.96 "Extra large" -> SetToolDefault(TextSize(96.0))
        option [ ] text.size.144 "Huge" -> SetToolDefault(TextSize(144.0))
      controls
        stepper text.size.custom "Custom text size in pixels" value=14 dec -> SetToolDefault(TextSize(13.0)) inc -> SetToolDefault(TextSize(15.0))
    dropdown text.font "Set default text font" shows text="Sans" font=Sans
      options list
        option [x] text.font.sans "Sans" text="Sans" font=Sans -> SetToolDefault(TextFont(Sans))
        option [ ] text.font.mono "Mono" text="Mono" font=Mono -> SetToolDefault(TextFont(Mono))
        option [ ] text.font.hand "Hand" text="Hand" font=Hand -> SetToolDefault(TextFont(Hand))
    ---
    swatches text.color Vivid/Ink: *neutral purple blue cyan green yellow orange red
    "#);
}

#[test]
fn the_sticky_tool_popup_sets_the_sticky_defaults() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddSticky);
    assert_popup_snapshot!(app, @r#"
    anchor toolbar gap=8
    dropdown sticky.size "Set default sticky text size" shows text="Small"
      options list
        option [x] sticky.size.14 "Small" -> SetToolDefault(StickySize(14.0))
        option [ ] sticky.size.32 "Medium" -> SetToolDefault(StickySize(32.0))
        option [ ] sticky.size.56 "Large" -> SetToolDefault(StickySize(56.0))
        option [ ] sticky.size.96 "Extra large" -> SetToolDefault(StickySize(96.0))
        option [ ] sticky.size.144 "Huge" -> SetToolDefault(StickySize(144.0))
      controls
        stepper sticky.size.custom "Custom text size in pixels" value=14 dec -> SetToolDefault(StickySize(13.0)) inc -> SetToolDefault(StickySize(15.0))
    dropdown sticky.font "Set default sticky text font" shows text="Sans" font=Sans
      options list
        option [x] sticky.font.sans "Sans" text="Sans" font=Sans -> SetToolDefault(StickyFont(Sans))
        option [ ] sticky.font.mono "Mono" text="Mono" font=Mono -> SetToolDefault(StickyFont(Mono))
        option [ ] sticky.font.hand "Hand" text="Hand" font=Hand -> SetToolDefault(StickyFont(Hand))
    ---
    swatches sticky.color Soft/Fill: neutral purple blue cyan green *yellow orange red
    "#);
}

#[test]
fn the_shape_tool_popup_sets_the_shape_defaults() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddShape);
    assert_popup_snapshot!(app, @r#"
    anchor toolbar gap=8
    dropdown shape.kind "Set default shape" shows icon=Shape(Rectangle)
      options grid(5)
        option [x] shape.kind.rectangle "Rectangle" icon=Shape(Rectangle) -> SetToolDefault(ShapeKind(Rectangle))
        option [ ] shape.kind.rounded "Rounded rectangle" icon=Shape(Rounded) -> SetToolDefault(ShapeKind(Rounded))
        option [ ] shape.kind.ellipse "Ellipse" icon=Shape(Ellipse) -> SetToolDefault(ShapeKind(Ellipse))
        option [ ] shape.kind.diamond "Diamond" icon=Shape(Diamond) -> SetToolDefault(ShapeKind(Diamond))
        option [ ] shape.kind.triangle "Triangle" icon=Shape(Triangle) -> SetToolDefault(ShapeKind(Triangle))
        option [ ] shape.kind.hexagon "Hexagon" icon=Shape(Hexagon) -> SetToolDefault(ShapeKind(Hexagon))
        option [ ] shape.kind.pill "Pill" icon=Shape(Pill) -> SetToolDefault(ShapeKind(Pill))
        option [ ] shape.kind.parallelogram "Parallelogram" icon=Shape(Parallelogram) -> SetToolDefault(ShapeKind(Parallelogram))
        option [ ] shape.kind.chevron "Chevron" icon=Shape(Chevron) -> SetToolDefault(ShapeKind(Chevron))
        option [ ] shape.kind.cylinder "Cylinder" icon=Shape(Cylinder) -> SetToolDefault(ShapeKind(Cylinder))
    ---
    dropdown shape.size "Set default label size" shows text="Small"
      options list
        option [x] shape.size.14 "Small" -> SetToolDefault(ShapeTextSize(14.0))
        option [ ] shape.size.32 "Medium" -> SetToolDefault(ShapeTextSize(32.0))
        option [ ] shape.size.56 "Large" -> SetToolDefault(ShapeTextSize(56.0))
        option [ ] shape.size.96 "Extra large" -> SetToolDefault(ShapeTextSize(96.0))
        option [ ] shape.size.144 "Huge" -> SetToolDefault(ShapeTextSize(144.0))
      controls
        stepper shape.size.custom "Custom text size in pixels" value=14 dec -> SetToolDefault(ShapeTextSize(13.0)) inc -> SetToolDefault(ShapeTextSize(15.0))
    ---
    swatches shape.color Soft/Fill: neutral purple blue cyan green yellow orange *red
    "#);
}

#[test]
fn the_draw_tool_popup_sets_the_brush_width_and_color() {
    let mut app = TestApp::empty();
    app.tool(Tool::Draw);
    assert_popup_snapshot!(app, @r#"
    anchor toolbar gap=8
    toggle [x] brush.pen "Pen" icon=BrushPen color=1 -> SetToolDefault(Brush(Pen))
    toggle [ ] brush.highlighter "Highlighter" icon=BrushHighlighter color=1 -> SetToolDefault(Brush(Highlight))
    ---
    toggle [x] width.thin "Set width to 2px" icon=StrokeThin -> SetToolDefault(DrawStrokeWidth(2.0))
    toggle [ ] width.thick "Set width to 4px" icon=StrokeThick -> SetToolDefault(DrawStrokeWidth(4.0))
    ---
    swatches draw.color Vivid/Ink: neutral purple blue cyan green yellow orange *red
    "#);
}

#[test]
fn the_draw_tool_popup_offers_the_highlighter_its_own_widths_and_pastels() {
    let mut app = TestApp::empty();
    app.act(Action::SetToolVariant(ToolDefaultPatch::Brush(
        BrushType::Highlight,
    )));
    assert_popup_snapshot!(app, @r#"
    anchor toolbar gap=8
    toggle [ ] brush.pen "Pen" icon=BrushPen color=1 -> SetToolDefault(Brush(Pen))
    toggle [x] brush.highlighter "Highlighter" icon=BrushHighlighter color=1 -> SetToolDefault(Brush(Highlight))
    ---
    toggle [x] width.thin "Set width to 8px" icon=StrokeThin -> SetToolDefault(DrawStrokeWidth(8.0))
    toggle [ ] width.thick "Set width to 16px" icon=StrokeThick -> SetToolDefault(DrawStrokeWidth(16.0))
    ---
    swatches draw.color Soft/Ink: neutral purple blue cyan green yellow orange *red
    "#);
}

#[test]
fn the_page_tool_has_no_popup_and_still_hides_the_selections() {
    let mut app =
        TestApp::with_entities([shape("s", specular_doc::Rect::new(0.0, 0.0, 100.0, 100.0))]);
    app.select(&["s"]);
    assert_ne!(app.popup_snapshot(), "none");
    app.tool(Tool::AddPage);
    assert_popup_snapshot!(app, @r#"
    anchor toolbar gap=8
    choices page.preset "Page size to add"
      options list
        option [x] page.preset.0 "Add iPhone SE" text="iPhone SE" trailing="375×667" -> SetToolDefault(PagePreset(0))
        option [ ] page.preset.1 "Add iPhone 14 Pro" text="iPhone Pro" trailing="393×852" -> SetToolDefault(PagePreset(1))
        option [ ] page.preset.2 "Add iPhone 14 Pro Max" text="iPhone Pro Max" trailing="430×932" -> SetToolDefault(PagePreset(2))
        option [ ] page.preset.9 "Add iPhone Duo (cover)" text="iPhone Duo (cover)" trailing="466×678" -> SetToolDefault(PagePreset(9))
        option [ ] page.preset.10 "Add iPhone Duo (open)" text="iPhone Duo (open)" trailing="626×890" -> SetToolDefault(PagePreset(10))
      options list
        option [ ] page.preset.3 "Add iPad Mini" text="iPad Mini" trailing="744×1133" -> SetToolDefault(PagePreset(3))
        option [ ] page.preset.4 "Add iPad Pro 11" text="iPad Pro 11" trailing="834×1194" -> SetToolDefault(PagePreset(4))
        option [ ] page.preset.5 "Add iPad Pro 12.9" text="iPad Pro 12.9" trailing="1024×1366" -> SetToolDefault(PagePreset(5))
      options list
        option [ ] page.preset.6 "Add Laptop" text="Laptop" trailing="1280×800" -> SetToolDefault(PagePreset(6))
        option [ ] page.preset.7 "Add Desktop" text="Desktop" trailing="1440×900" -> SetToolDefault(PagePreset(7))
        option [ ] page.preset.8 "Add Desktop XL" text="Desktop XL" trailing="1920×1080" -> SetToolDefault(PagePreset(8))
      options list
        option [ ] page.preset.custom "Add custom" text="Custom" -> SetToolDefault(PageCustom)
    "#);
}

#[test]
fn a_tool_with_a_popup_wins_over_the_selection_and_keeps_it_while_drawing() {
    let mut app =
        TestApp::with_entities([shape("s", specular_doc::Rect::new(0.0, 0.0, 100.0, 100.0))]);
    app.select(&["s"]);
    app.tool(Tool::Draw);
    let at_rest = app.popup_snapshot();
    assert!(at_rest.starts_with("anchor toolbar"));
    app.press((300.0, 300.0)).drag_to((340.0, 330.0));
    assert_eq!(app.popup_snapshot(), at_rest);
}
