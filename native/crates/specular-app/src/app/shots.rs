//! Pictures for the API: the window, a region of the canvas, or one page.
//!
//! All of them go through the compositor into a texture of their own, so
//! the window is not touched and may be covered or minimised. A region and
//! a page are drawn at their own size whatever the window's zoom, which is
//! what the Electron app's `capturePage` and region capture give at zoom 1.
//! A page is drawn from the frame the compositor already holds, so a page
//! the level-of-detail policy has slowed or shrunk is shown as the canvas
//! would show it, not painted afresh.

use base64::Engine as _;
use glam::Vec2;
use serde_json::{Value, json};
use specular_api::{Screenshot, ShotArea};
use specular_core::Camera;
use specular_doc::{EntityId, Rect};
use specular_interact::{Action, Event, update};
use specular_scene::Draw;

use super::runtime::{Runtime, ShellWindow};

/// The longest side of a picture, in logical pixels. A larger area is
/// drawn zoomed out to fit, so one request cannot ask for a texture the
/// GPU will not make.
const MAX_SIDE: f64 = 4096.0;
/// Room above a page for its title line, in canvas units, when a page is
/// drawn with its chrome.
const TITLE_ROOM: f64 = 28.0;

type Png = (Vec<u8>, u32, u32);

impl<W: ShellWindow> Runtime<W> {
    /// Draws what `shot` asks for; the body of the API's answer.
    pub(super) fn shoot(&mut self, shot: &Screenshot) -> Result<Value, String> {
        let (png, width, height) = match &shot.area {
            ShotArea::Window => self.shoot_window()?,
            ShotArea::Canvas(rect) => self.shoot_canvas(*rect, None)?,
            ShotArea::Page {
                page,
                chrome,
                padding,
            } => {
                let placement = (self.app.page_placement(page))
                    .ok_or_else(|| format!("Page not found: {page}"))?;
                let rect = placement.rect;
                if *chrome {
                    let around = Rect::new(
                        rect.x - padding,
                        rect.y - padding - TITLE_ROOM,
                        rect.width + 2.0 * padding,
                        rect.height + 2.0 * padding + TITLE_ROOM,
                    );
                    self.shoot_canvas(around, None)?
                } else {
                    self.shoot_canvas(rect, Some(page))?
                }
            }
        };
        let mut body = json!({ "mimeType": "image/png", "width": width, "height": height });
        match &shot.path {
            Some(path) => {
                std::fs::write(path, png)
                    .map_err(|error| format!("writing {}: {error}", path.display()))?;
                body["path"] = json!(path);
            }
            None => {
                body["base64"] = json!(base64::engine::general_purpose::STANDARD.encode(png));
            }
        }
        Ok(body)
    }

    fn shoot_window(&mut self) -> Result<Png, String> {
        let gpu = self.gpu.as_mut().ok_or("the window is not open yet")?;
        let scene = specular_scene::view(&self.app, gpu.logical_viewport());
        let hosts = &self.hosts;
        let page_of = |entity: &EntityId| hosts.get(entity).map(|host| host.page);
        gpu.capture(self.app.session().camera, &scene, &page_of)
            .map_err(|error| format!("{error:#}"))
    }

    /// Draws the canvas rect `rect`. With `only`, that page's pixels and
    /// nothing else: no border, title, selection or item lying over it.
    fn shoot_canvas(&mut self, rect: Rect, only: Option<&EntityId>) -> Result<Png, String> {
        if !(rect.width >= 1.0 && rect.height >= 1.0) {
            return Err("the area to draw is empty".to_owned());
        }
        let zoom = (MAX_SIDE / rect.width.max(rect.height)).min(1.0);
        let camera = Camera {
            pan: Vec2::new((-rect.x * zoom) as f32, (-rect.y * zoom) as f32),
            zoom: zoom as f32,
        };
        let viewport = Vec2::new((rect.width * zoom) as f32, (rect.height * zoom) as f32);
        // The scene is built for a camera, so the picture's camera goes on
        // a copy of the app: the user's view does not move.
        let mut app = self.app.clone();
        update(&mut app, Event::ViewportResized(viewport));
        update(&mut app, Event::Action(Action::SetCamera(camera)));
        let scene = match only {
            None => specular_scene::view(&app, viewport),
            Some(page) => {
                let mut scene = specular_scene::view_without_chrome(&app, viewport);
                (scene.items)
                    .retain(|item| matches!(&item.draw, Draw::Page(draw) if draw.page == *page));
                for item in &mut scene.items {
                    if let Draw::Page(draw) = &mut item.draw {
                        draw.corner_radius = 0.0;
                    }
                }
                scene
            }
        };
        let gpu = self.gpu.as_mut().ok_or("the window is not open yet")?;
        let hosts = &self.hosts;
        let page_of = |entity: &EntityId| hosts.get(entity).map(|host| host.page);
        gpu.capture_area(camera, viewport, &scene, &page_of)
            .map_err(|error| format!("{error:#}"))
    }
}
