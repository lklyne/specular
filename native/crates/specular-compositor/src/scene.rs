//! What the caller asks [`Compositor::render`](crate::Compositor::render) to
//! draw, and what it reports back.

use std::time::Duration;

use glam::Vec2;
use specular_core::{Camera, CanvasRect, PageId};

/// One page to draw this frame, in paint order (back to front).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageDraw {
    /// Which page's latest frame to draw.
    pub page: PageId,
    /// Where, in canvas space.
    pub rect: CanvasRect,
}

/// Dot-grid background parameters (canvas-space spacing, like canvas-bg).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotGrid {
    /// Distance between dots in canvas units (`GRID_SIZE`). The on-screen
    /// step doubles when zoomed out so dots never get denser than 8px.
    pub spacing: f32,
    /// Dot radius in screen (logical) pixels, snapped to device pixels.
    pub radius: f32,
    /// Background colour, linear RGBA.
    pub background: [f32; 4],
    /// Dot colour, linear RGBA.
    pub dot: [f32; 4],
}

impl Default for DotGrid {
    fn default() -> Self {
        Self {
            spacing: 20.0,
            radius: 0.7,
            background: [0.91, 0.91, 0.91, 1.0],
            dot: [0.45, 0.45, 0.45, 1.0],
        }
    }
}

/// Where a shape sits and how big it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeExtent {
    /// A rect in canvas units; it scales with zoom.
    Canvas(CanvasRect),
    /// A box of `size` logical pixels centred on a canvas-space `anchor`;
    /// its size does not change with zoom.
    Screen {
        /// Centre, in canvas units.
        anchor: Vec2,
        /// Width and height in logical pixels.
        size: Vec2,
    },
}

/// An untextured rounded rectangle drawn above every page.
///
/// The fill covers the rect exactly. The stroke sits **outside** the rect
/// edge: it occupies the band from the edge out to `stroke_width`, so an
/// outline never covers the content it frames and a page border can hug the
/// page without overlapping it. A screen-sized shape's footprint therefore
/// grows by `stroke_width` on every side. The stroke follows the rounded
/// corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeDraw {
    /// Position and size.
    pub extent: ShapeExtent,
    /// Corner radius, in the units of `extent` (canvas units scale with
    /// zoom, logical pixels do not). Clamped to half the shorter side; a
    /// circle is a square whose radius is half its size.
    pub corner_radius: f32,
    /// Fill colour, linear RGBA with straight alpha.
    pub fill: [f32; 4],
    /// Stroke colour, linear RGBA with straight alpha.
    pub stroke: [f32; 4],
    /// Stroke width in logical pixels at any zoom.
    pub stroke_width: f32,
}

/// Everything [`Compositor::render`](crate::Compositor::render) needs for one
/// window frame.
#[derive(Debug, Clone, Copy)]
pub struct SceneView<'a> {
    /// Canvas camera.
    pub camera: Camera,
    /// Viewport size in logical pixels.
    pub viewport: Vec2,
    /// Physical pixels per logical pixel (window scale factor).
    pub scale_factor: f32,
    /// Pages in paint order.
    pub pages: &'a [PageDraw],
    /// Shapes in paint order, drawn above all pages.
    pub shapes: &'a [ShapeDraw],
    /// Background grid.
    pub grid: DotGrid,
}

/// Per-frame counters, comparable to the Electron lab's benchmark fields
/// and the page host's `PageHostStats`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RenderStats {
    /// Visible pages that had no frame yet (`framesWithoutTexture`).
    pub pages_without_texture: u32,
    /// Pages whose current texture came from a CPU upload (non-representative).
    pub cpu_textures: u32,
    /// Shapes drawn, after culling those outside the viewport.
    pub shapes_drawn: u32,
    /// Longest wait, among frames shown for the first time this render,
    /// between the source receiving the paint and this frame's submit.
    pub max_paint_to_submit: Option<Duration>,
    /// View-layer frames ingested since the previous render.
    pub frames_received: u32,
    /// Popup-layer frames ingested since the previous render.
    pub popup_frames: u32,
    /// Paints the source refused at the outstanding-texture cap since the
    /// previous render.
    pub frames_dropped_for_pool_pressure: u32,
    /// Shared textures held after this render, summed over pages.
    pub outstanding_textures: u32,
    /// Shared textures held by the page holding the most.
    pub max_outstanding_textures: u32,
}
