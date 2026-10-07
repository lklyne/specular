//! Candidate A: vello draws every canvas item, with parley laying out the text.
//!
//! `--retained` encodes the world once and replays it under the camera
//! transform; the default re-encodes the visible items every frame, which is
//! what a `view(&App) -> Scene` loop does. `--lod` skips text too small to read.

use std::borrow::Cow;

use anyhow::{Result, anyhow};
use bakeoff_scene::{
    Camera, Gpu, World,
    camera::VIEWPORT,
    harness::{self, Candidate},
    world::{self, Arrow, Note, Stroke},
};
use bakeoff_ui::Panels;
use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily, Layout, LayoutContext, LineHeight,
    PositionedLayoutItem, StyleProperty,
};
use vello::{
    AaConfig, AaSupport, Glyph, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{self, Affine, BezPath, Cap, CubicBez, Join, RoundedRect},
    peniko::{Color, Fill},
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

struct VelloCandidate {
    renderer: Renderer,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    scene: Scene,
    layouts: Vec<Layout<()>>,
    stroke_paths: Vec<BezPath>,
    /// The whole world encoded once in canvas units, when `--retained`.
    retained: Option<Scene>,
    lod: bool,
}

impl VelloCandidate {
    fn new(gpu: &Gpu, world: &World, retained: bool) -> Result<Self> {
        let renderer = Renderer::new(
            &gpu.device,
            RendererOptions {
                antialiasing_support: AaSupport::area_only(),
                ..RendererOptions::default()
            },
        )
        .map_err(|error| anyhow!("vello renderer: {error}"))?;
        let target = gpu.target(
            FORMAT,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            1,
        );
        let mut candidate = Self {
            renderer,
            view: target.create_view(&wgpu::TextureViewDescriptor::default()),
            target,
            scene: Scene::new(),
            layouts: layouts(world),
            stroke_paths: world.strokes.iter().map(stroke_path).collect(),
            retained: None,
            lod: harness::flag("--lod"),
        };
        if retained {
            let mut scene = Scene::new();
            candidate.encode(&mut scene, world, None);
            candidate.retained = Some(scene);
        }
        Ok(candidate)
    }

    /// Encodes the world into `scene`. With a camera, items are transformed to
    /// the screen and culled; without one they stay in canvas units.
    fn encode(&self, scene: &mut Scene, world: &World, camera: Option<Camera>) {
        let to_screen = camera.map_or(Affine::IDENTITY, camera_transform);
        let sees = |origin, size| camera.is_none_or(|camera| camera.sees(origin, size));
        let text = camera.is_none_or(|camera| camera.shows_text(self.lod));

        for (note, layout) in world.notes.iter().zip(&self.layouts) {
            if sees(note.origin, world::NOTE_SIZE) {
                draw_note(scene, to_screen, note, text.then_some(layout));
            }
        }
        for (stroke, path) in world.strokes.iter().zip(&self.stroke_paths) {
            let (origin, size) = world::bounds(&stroke.points, stroke.width);
            if sees(origin, size) {
                let style = kurbo::Stroke::new(f64::from(stroke.width))
                    .with_caps(Cap::Round)
                    .with_join(Join::Round);
                scene.stroke(&style, to_screen, rgb(stroke.color), None, path);
            }
        }
        for arrow in &world.arrows {
            let (origin, size) = world::bounds(&arrow.p, arrow.width * 5.0);
            if sees(origin, size) {
                draw_arrow(scene, to_screen, arrow);
            }
        }
    }
}

impl Candidate for VelloCandidate {
    fn frame(&mut self, gpu: &Gpu, world: &World, camera: Camera) -> Result<()> {
        let mut scene = std::mem::take(&mut self.scene);
        scene.reset();
        match &self.retained {
            Some(retained) => scene.append(retained, Some(camera_transform(camera))),
            None => self.encode(&mut scene, world, Some(camera)),
        }
        self.scene = scene;
        self.renderer
            .render_to_texture(
                &gpu.device,
                &gpu.queue,
                &self.scene,
                &self.view,
                &RenderParams {
                    base_color: rgb(world::BACKGROUND),
                    width: VIEWPORT[0],
                    height: VIEWPORT[1],
                    antialiasing_method: AaConfig::Area,
                },
            )
            .map_err(|error| anyhow!("vello render: {error}"))
    }

    fn target(&self) -> &wgpu::Texture {
        &self.target
    }
}

/// Shapes and wraps every note's text once, in canvas units.
fn layouts(world: &World) -> Vec<Layout<()>> {
    let mut fonts = FontContext::new();
    let mut context = LayoutContext::new();
    let width = world::NOTE_SIZE[0] - 2.0 * world::NOTE_PADDING;
    world
        .notes
        .iter()
        .map(|note| {
            let mut builder = context.ranged_builder(&mut fonts, &note.text, 1.0, false);
            builder.push_default(StyleProperty::FontFamily(FontFamily::Source(
                Cow::Borrowed(world::FONT_FAMILY),
            )));
            builder.push_default(StyleProperty::FontSize(world::FONT_SIZE));
            builder.push_default(LineHeight::Absolute(world::LINE_HEIGHT));
            let mut layout: Layout<()> = builder.build(&note.text);
            layout.break_all_lines(Some(width));
            layout.align(Alignment::Start, AlignmentOptions::default());
            layout
        })
        .collect()
}

fn draw_note(scene: &mut Scene, to_screen: Affine, note: &Note, layout: Option<&Layout<()>>) {
    let [x, y] = note.origin.map(f64::from);
    let [width, height] = world::NOTE_SIZE.map(f64::from);
    let body = RoundedRect::new(x, y, x + width, y + height, f64::from(world::NOTE_RADIUS));
    scene.fill(Fill::NonZero, to_screen, rgb(note.color), None, &body);

    let Some(layout) = layout else { return };
    let padding = f64::from(world::NOTE_PADDING);
    let text_transform = to_screen * Affine::translate((x + padding, y + padding));
    for line in layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(run) = item else {
                continue;
            };
            scene
                .draw_glyphs(run.run().font())
                .font_size(run.run().font_size())
                .transform(text_transform)
                .brush(rgb(world::TEXT_COLOR))
                .draw(
                    Fill::NonZero,
                    run.positioned_glyphs().map(|glyph| Glyph {
                        id: glyph.id,
                        x: glyph.x,
                        y: glyph.y,
                    }),
                );
        }
    }
}

fn draw_arrow(scene: &mut Scene, to_screen: Affine, arrow: &Arrow) {
    let [a, b, c, d] = arrow.p.map(point);
    let style = kurbo::Stroke::new(f64::from(arrow.width)).with_caps(Cap::Round);
    scene.stroke(
        &style,
        to_screen,
        rgb(arrow.color),
        None,
        &CubicBez::new(a, b, c, d),
    );

    let [tip, left, right] = arrow.head().map(point);
    let mut head = BezPath::new();
    head.move_to(tip);
    head.line_to(left);
    head.line_to(right);
    head.close_path();
    scene.fill(Fill::NonZero, to_screen, rgb(arrow.color), None, &head);
}

fn stroke_path(stroke: &Stroke) -> BezPath {
    let mut path = BezPath::new();
    for (index, at) in stroke.points.iter().enumerate() {
        if index == 0 {
            path.move_to(point(*at))
        } else {
            path.line_to(point(*at))
        }
    }
    path
}

fn camera_transform(camera: Camera) -> Affine {
    let [x, y] = camera.offset().map(f64::from);
    Affine::translate((x, y)) * Affine::scale(f64::from(camera.zoom))
}

fn point(at: [f32; 2]) -> kurbo::Point {
    kurbo::Point::new(f64::from(at[0]), f64::from(at[1]))
}

fn rgb(color: [u8; 3]) -> Color {
    Color::from_rgb8(color[0], color[1], color[2])
}

fn main() -> Result<()> {
    let retained = harness::flag("--retained");
    let name = &harness::run_name("vello");
    let gpu = Gpu::headless()?;
    let world = World::build();
    let mut candidate = VelloCandidate::new(&gpu, &world, retained)?;

    harness::golden_images(name, &gpu, &world, &mut candidate)?;
    let mut panels = Panels::new(&gpu, FORMAT);
    harness::panel_overlay(name, &gpu, &world, &mut candidate, |gpu, view, zoom| {
        panels.paint(gpu, view, zoom);
    })?;
    harness::timings(name, &gpu, &world, &mut candidate)
}
