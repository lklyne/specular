//! What a runtime needs from the window it draws into.

use glam::Vec2;
use specular_compositor::{Compositor, SceneStats};
use specular_core::{Camera, PageId};
use specular_doc::EntityId;
use specular_interact::Cursor;
use specular_scene::Scene;

/// The backend page behind a page entity, for a renderer.
pub type PageOf<'a> = &'a dyn Fn(&EntityId) -> Option<PageId>;

/// What a [`Runtime`] needs of the window it draws into: the compositor,
/// a frame on screen, and the few window calls an effect asks for.
pub trait ShellWindow {
    /// The compositor drawing into this window.
    fn compositor(&self) -> &Compositor;

    /// The compositor, to upload to.
    fn compositor_mut(&mut self) -> &mut Compositor;

    /// Device pixels per logical pixel.
    fn scale_factor(&self) -> f32;

    /// The canvas viewport in logical pixels.
    fn logical_viewport(&self) -> Vec2;

    /// Draws `scene` into a texture of `viewport` logical pixels at the
    /// window's scale, whatever size the window is, and returns the PNG
    /// with its size in device pixels.
    fn capture_area(
        &mut self,
        camera: Camera,
        viewport: Vec2,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)>;

    /// Renders and presents one frame of `scene`; `None` when there was no
    /// frame to draw into. The scene is the window's to rework: one whose
    /// viewport is only part of its surface moves the screen-space items.
    fn render(
        &mut self,
        camera: Camera,
        zooming: bool,
        scene: &mut Scene,
        page_of: PageOf<'_>,
    ) -> Option<SceneStats>;

    /// Draws `scene` into a texture and returns the PNG with its size in
    /// device pixels. The window itself is not touched.
    fn capture(
        &mut self,
        camera: Camera,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)>;

    /// Whether the input method may compose into the canvas.
    fn set_ime_allowed(&self, allowed: bool);

    /// Where the caret is, in logical pixels of the viewport, for the
    /// input method's candidate window.
    fn set_ime_cursor_area(&self, origin: Vec2, size: Vec2);

    /// The pointer's shape over the canvas.
    fn set_cursor(&self, cursor: Cursor);

    /// Gives the keys to the field `field` of the chrome, with its text
    /// selected. Only a window that draws the chrome itself is asked.
    fn edit_field(&self, field: &specular_interact::ControlId);

    /// The window's title, and whether it shows unsaved changes.
    fn set_title(&self, title: &str, unsaved: bool);
}
