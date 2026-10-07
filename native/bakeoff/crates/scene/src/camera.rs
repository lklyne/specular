//! Canvas-to-screen mapping and the camera paths the candidates are run through.

use crate::World;

pub const VIEWPORT: [u32; 2] = [1600, 1000];
/// Zoom levels the golden images are captured at.
pub const GOLDEN_ZOOMS: [f32; 4] = [0.05, 0.25, 1.0, 3.0];
const SWEEP_ZOOM: [f32; 2] = [0.02, 3.0];

#[derive(Clone, Copy)]
pub struct Camera {
    /// Canvas point at the middle of the viewport.
    pub center: [f32; 2],
    pub zoom: f32,
}

impl Camera {
    /// Screen position of canvas origin; `screen = canvas * zoom + offset`.
    pub fn offset(&self) -> [f32; 2] {
        [
            VIEWPORT[0] as f32 * 0.5 - self.center[0] * self.zoom,
            VIEWPORT[1] as f32 * 0.5 - self.center[1] * self.zoom,
        ]
    }

    pub fn to_screen(&self, point: [f32; 2]) -> [f32; 2] {
        let offset = self.offset();
        [
            point[0] * self.zoom + offset[0],
            point[1] * self.zoom + offset[1],
        ]
    }

    /// Whether a canvas rect (`origin`, `size`) touches the viewport.
    pub fn sees(&self, origin: [f32; 2], size: [f32; 2]) -> bool {
        let min = self.to_screen(origin);
        let max = self.to_screen([origin[0] + size[0], origin[1] + size[1]]);
        max[0] >= 0.0
            && max[1] >= 0.0
            && min[0] <= VIEWPORT[0] as f32
            && min[1] <= VIEWPORT[1] as f32
    }

    /// Whether note text is worth drawing. With `lod` on, text whose em is
    /// under 2.5 screen pixels is skipped: it reads as a tint either way.
    pub fn shows_text(&self, lod: bool) -> bool {
        !lod || self.zoom * crate::world::FONT_SIZE >= 2.5
    }

    pub fn golden(world: &World, zoom: f32) -> Self {
        Self {
            center: world.focus(),
            zoom,
        }
    }

    /// Zooms out to in and back while orbiting the middle of the world, so every
    /// frame changes both scale and translation.
    pub fn sweep(world: &World, frames: usize) -> impl Iterator<Item = Self> + '_ {
        let ratio = SWEEP_ZOOM[1] / SWEEP_ZOOM[0];
        (0..frames).map(move |frame| {
            let t = frame as f32 / (frames - 1).max(1) as f32;
            let there_and_back = 1.0 - (2.0 * t - 1.0).abs();
            let angle = t * std::f32::consts::TAU;
            Self {
                center: [
                    world.size[0] * 0.5 + angle.cos() * 500.0,
                    world.size[1] * 0.5 + angle.sin() * 500.0,
                ],
                zoom: SWEEP_ZOOM[0] * ratio.powf(there_and_back),
            }
        })
    }

    /// A pan at one zoom: scale is constant, translation changes every frame.
    pub fn pan(world: &World, zoom: f32, frames: usize) -> impl Iterator<Item = Self> + '_ {
        let focus = world.focus();
        (0..frames).map(move |frame| Self {
            center: [
                focus[0] + frame as f32 * 3.0 / zoom,
                focus[1] + frame as f32 / zoom,
            ],
            zoom,
        })
    }
}
