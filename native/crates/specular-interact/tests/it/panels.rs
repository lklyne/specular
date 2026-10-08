//! The item popup for text, shapes, drawings and groups: the controls in
//! order, their state, and the action each carries.

use specular_doc::{
    BrushType, Color, ColorPreset, Drawing, Entity, FillStyle, JsonMap, Kind, Point, Rect, Shape,
    ShapeKind, Stroke,
};
use specular_testkit::{TestApp, assert_popup_snapshot, sticky};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const RED: Color = Color::Preset(ColorPreset::Red);

/// A drawing of one stroke.
fn ink(color: Color, width: f64, brush: Option<BrushType>) -> Entity {
    let stroke = Stroke {
        id: "ink".to_owned(),
        color,
        width,
        points: vec![Point::new(120.0, 120.0), Point::new(160.0, 150.0)],
        brush,
        extra: JsonMap::new(),
    };
    Entity::new(
        "d",
        Rect::new(116.0, 116.0, 48.0, 38.0),
        Kind::Drawing(Drawing {
            strokes: vec![stroke],
        }),
    )
}

#[test]
fn a_sticky_has_size_font_color_and_no_formatting_until_edited() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    app.select(&["a"]);
    assert_popup_snapshot!(app, @r#"
    anchor canvas 100,100 200x100 Above Center gap=14
    dropdown text.size "Set text size" shows text="Small"
      options list
        option [x] text.size.14 "Small" -> SetProperty(TextSize(14.0))
        option [ ] text.size.32 "Medium" -> SetProperty(TextSize(32.0))
        option [ ] text.size.56 "Large" -> SetProperty(TextSize(56.0))
        option [ ] text.size.96 "Extra large" -> SetProperty(TextSize(96.0))
        option [ ] text.size.144 "Huge" -> SetProperty(TextSize(144.0))
      controls
        stepper text.size.custom "Custom text size in pixels" value=14 dec -> SetProperty(TextSize(13.0)) inc -> SetProperty(TextSize(15.0))
    dropdown text.font "Set text font" shows text="Sans" font=Sans
      options list
        option [x] text.font.sans "Sans" text="Sans" font=Sans -> SetProperty(TextFont(Sans))
        option [ ] text.font.mono "Mono" text="Mono" font=Mono -> SetProperty(TextFont(Mono))
        option [ ] text.font.hand "Hand" text="Hand" font=Hand -> SetProperty(TextFont(Hand))
    ---
    dropdown text.color "Set sticky note color" shows hollow
      controls
        swatches text.color.swatches Soft/Fill: neutral purple blue cyan green yellow orange red
    ---
    button item.annotate "Annotate sticky note" icon=Annotate -> AnnotateSelection
    button item.focus "Focus sticky note" icon=Focus -> FocusSelection
    "#);
}

#[test]
fn a_transparent_shape_shows_the_clear_swatch_and_a_borderless_one_disables_the_border_color() {
    let clear = Entity::new(
        "s",
        A,
        Kind::Shape(Shape {
            fill_style: Some(FillStyle::None),
            border_style: Some(specular_doc::BorderStyle::None),
            ..Shape::new(ShapeKind::Rectangle)
        }),
    );
    let mut app = TestApp::with_entities([clear]);
    app.select(&["s"]);
    assert_popup_snapshot!(app, @r#"
    anchor canvas 100,100 200x100 Above Center gap=14
    dropdown shape.kind "Set shape" shows icon=Shape(Rectangle)
      options grid(5)
        option [x] shape.kind.rectangle "Rectangle" icon=Shape(Rectangle) -> SetProperty(ShapeKind(Rectangle))
        option [ ] shape.kind.rounded "Rounded rectangle" icon=Shape(Rounded) -> SetProperty(ShapeKind(Rounded))
        option [ ] shape.kind.ellipse "Ellipse" icon=Shape(Ellipse) -> SetProperty(ShapeKind(Ellipse))
        option [ ] shape.kind.diamond "Diamond" icon=Shape(Diamond) -> SetProperty(ShapeKind(Diamond))
        option [ ] shape.kind.triangle "Triangle" icon=Shape(Triangle) -> SetProperty(ShapeKind(Triangle))
        option [ ] shape.kind.hexagon "Hexagon" icon=Shape(Hexagon) -> SetProperty(ShapeKind(Hexagon))
        option [ ] shape.kind.pill "Pill" icon=Shape(Pill) -> SetProperty(ShapeKind(Pill))
        option [ ] shape.kind.parallelogram "Parallelogram" icon=Shape(Parallelogram) -> SetProperty(ShapeKind(Parallelogram))
        option [ ] shape.kind.chevron "Chevron" icon=Shape(Chevron) -> SetProperty(ShapeKind(Chevron))
        option [ ] shape.kind.cylinder "Cylinder" icon=Shape(Cylinder) -> SetProperty(ShapeKind(Cylinder))
    ---
    dropdown shape.size "Set label size" shows text="Small"
      options list
        option [x] shape.size.14 "Small" -> SetProperty(TextSize(14.0))
        option [ ] shape.size.32 "Medium" -> SetProperty(TextSize(32.0))
        option [ ] shape.size.56 "Large" -> SetProperty(TextSize(56.0))
        option [ ] shape.size.96 "Extra large" -> SetProperty(TextSize(96.0))
        option [ ] shape.size.144 "Huge" -> SetProperty(TextSize(144.0))
      controls
        stepper shape.size.custom "Custom text size in pixels" value=14 dec -> SetProperty(TextSize(13.0)) inc -> SetProperty(TextSize(15.0))
    ---
    dropdown shape.align "Text alignment" shows icon=AlignCenter
      options row
        option [ ] shape.align.left "Align text left" icon=AlignLeft -> SetProperty(TextAlign(Left))
        option [x] shape.align.center "Align text center" icon=AlignCenter -> SetProperty(TextAlign(Center))
        option [ ] shape.align.right "Align text right" icon=AlignRight -> SetProperty(TextAlign(Right))
    ---
    dropdown shape.color "Set shape color" shows icon=Ban
      controls
        swatches shape.color.swatches Soft/Fill: *transparent neutral purple blue cyan green yellow orange red
    ---
    dropdown shape.border "Border" shows icon=Border
      controls
        toggle [ ] shape.border.solid "Solid" icon=LineSolid text="Solid" -> SetProperty(BorderStyle(Solid))
        toggle [ ] shape.border.dashed "Dashed" icon=LineDashed text="Dashed" -> SetProperty(BorderStyle(Dashed))
        toggle [x] shape.border.none "None" icon=Ban text="None" -> SetProperty(BorderStyle(None))
        toggle [ ] shape.border.w1 "Set border width to 1px" text="1" disabled -> SetProperty(StrokeWidth(1.0))
        toggle [ ] shape.border.w2 "Set border width to 2px" text="2" disabled -> SetProperty(StrokeWidth(2.0))
        toggle [ ] shape.border.w3 "Set border width to 3px" text="3" disabled -> SetProperty(StrokeWidth(3.0))
        toggle [ ] shape.border.w4 "Set border width to 4px" text="4" disabled -> SetProperty(StrokeWidth(4.0))
      controls
        swatches shape.border.color Soft/Fill disabled: neutral purple blue cyan green yellow orange red
    ---
    button item.annotate "Annotate shape" icon=Annotate -> AnnotateSelection
    button item.focus "Focus shape" icon=Focus -> FocusSelection
    "#);
}

#[test]
fn a_drawing_has_brush_width_and_color() {
    let mut app = TestApp::with_entities([ink(RED, 8.0, Some(BrushType::Highlight))]);
    app.select(&["d"]);
    assert_popup_snapshot!(app, @r#"
    anchor canvas 116,116 48x38 Above Center gap=14
    toggle [ ] brush.pen "Pen" icon=BrushPen color=1 -> SetProperty(Brush(Pen))
    toggle [x] brush.highlighter "Highlighter" icon=BrushHighlighter color=1 -> SetProperty(Brush(Highlight))
    ---
    toggle [x] width.thin "Set width to 8px" icon=StrokeThin -> SetProperty(StrokeWidth(8.0))
    toggle [ ] width.thick "Set width to 16px" icon=StrokeThick -> SetProperty(StrokeWidth(16.0))
    ---
    dropdown drawing.color "Set drawing color" shows color=1
      controls
        swatches drawing.color.swatches Soft/Ink: neutral purple blue cyan green yellow orange *red
    ---
    button item.annotate "Annotate drawing" icon=Annotate -> AnnotateSelection
    button item.focus "Focus drawing" icon=Focus -> FocusSelection
    "#);
}
