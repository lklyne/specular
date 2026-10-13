//! Pages: the live frame, and around it a border or a device.

use specular_core::CssSize;
use specular_doc::{DeviceArt, DeviceFrameColor, DeviceKind, DeviceShell, Entity, Page};
use specular_interact::{Appearance, within_safe_area};

use super::frame::{Frame, canvas_rect, carried};
use crate::{
    Color, Colors, ImageDraw, ImageId, Item, PageDraw, Point, Rect, RectDraw, Scene, ShadowDraw,
    Stroke, StrokeAlign,
};

/// Corner radius of a page's frame, in canvas units.
pub(crate) const CORNER_RADIUS: f32 = 8.0;
const BORDER_WIDTH: f32 = 1.0;

/// The drop and blur of a device frame's shadow, in canvas units.
const SHELL_SHADOW: f32 = 16.0;
/// The ring in the bezel around the screen, in logical pixels.
const SCREEN_RING_WIDTH: f32 = 1.5;
/// A phone's notch: its size, its corner radius and its gap from the top
/// of the screen, in canvas units.
const NOTCH: (f32, f32, f32, f32) = (126.0, 37.0, 18.5, 8.0);
/// A home button's diameter and the width of its ring, in canvas units.
const HOME_BUTTON: (f32, f32) = (60.0, 3.0);
/// How far the dark under a realistic frame's screen reaches past the
/// page, in canvas units, so no light shows where the two edges meet.
const SCREEN_BACKING: f32 = 1.0;

/// How a turning page's parts are drawn: carried round the centre of its
/// shell and turned with it. A page that is not turning stays as it is.
#[derive(Debug, Clone, Copy)]
struct Carry {
    angle: f32,
    /// How far along the turn is, 1 for a page that is not turning.
    progress: f32,
    pivot: Point,
    pivot_on_screen: Point,
}

impl Carry {
    fn of(frame: &Frame<'_>, entity: &Entity) -> Self {
        let turning = frame.app.turning(&entity.id);
        let pivot = turning.map_or(glam::DVec2::ZERO, |turning| turning.pivot);
        Self {
            angle: turning.map_or(0.0, |turning| turning.angle),
            progress: turning.map_or(1.0, |turning| turning.progress.clamp(0.0, 1.0)),
            pivot: Point::new(pivot.x as f32, pivot.y as f32),
            pivot_on_screen: frame.screen_point(pivot),
        }
    }

    fn turning(&self) -> bool {
        self.angle != 0.0
    }

    /// An item in canvas space as a part of the page.
    fn canvas(&self, rect: Rect, draw: impl FnOnce(Rect) -> Item) -> Item {
        draw(carried(rect, self.pivot, self.angle)).turned(self.angle)
    }

    /// An image in canvas space as a part of the page, itself turned by
    /// `turn` in the page.
    fn image(&self, image: ImageId, rect: Rect, turn: f32) -> Item {
        let rect = carried(rect, self.pivot, self.angle);
        Item::canvas(ImageDraw::new(image, rect)).turned(self.angle + turn)
    }

    /// An item in screen space as a part of the page.
    fn screen(&self, rect: Rect, draw: impl FnOnce(Rect) -> Item) -> Item {
        draw(carried(rect, self.pivot_on_screen, self.angle)).turned(self.angle)
    }
}

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, page: &Page, scene: &mut Scene) {
    let screen = canvas_rect(entity.rect);
    // A screenshot of the page is the page alone, so the frame is chrome.
    let shell = page.shell().filter(|_| frame.chrome);
    let outer = shell.map_or(screen, |shell| shell_rect(screen, &shell));
    let carry = Carry::of(frame, entity);
    // A page that fills the view is at 100%, so its radius in screen pixels
    // is its radius on the canvas.
    let filling = frame.app.fill_corner_radius(&entity.id);
    let plain_radius = filling.unwrap_or(CORNER_RADIUS);
    let outer_radius = shell.map_or(plain_radius, |shell| shell.corner_radius as f32);
    // A realistic frame is its device's drawing once the shell has it.
    // Until then, and for a clay frame, the shell is drawn here.
    let color = frame.app.device_frame_color(page);
    let art = shell.and_then(|shell| {
        let image = frame.app.device_frame_image(shell.art?, color)?;
        Some((shell.art?, ImageId(image.texture(0))))
    });
    // The device is in its own colour's palette, whatever the app's theme.
    let device = Colors::of(match color {
        DeviceFrameColor::Light => Appearance::Light,
        DeviceFrameColor::Dark => Appearance::Dark,
    });
    if let Some(shell) = &shell {
        under_page(device, &carry, screen, shell, art.is_some(), scene);
    }
    let screen_radius = shell.map_or(plain_radius, |shell| shell.screen_corner_radius as f32);
    // A frame painted before the turn does not cover the screen on its way
    // round, and a page with no bezel has nothing else under it.
    if carry.turning() && shell.is_none() {
        scene.push(carry.canvas(screen, |rect| {
            Item::canvas(
                RectDraw::filled(rect, frame.colors.card).with_corner_radius(screen_radius),
            )
        }));
    }
    // A document laid out inside its screen's safe area leaves strips of
    // the screen bare, which are its colour as a phone paints them.
    let inset = frame.app.page_inset(&entity.id);
    let document = canvas_rect(within_safe_area(entity.rect, inset));
    let inside = document != screen;
    if inside {
        let [r, g, b] = frame.app.page_surround(&entity.id);
        scene.push(carry.canvas(screen, |rect| {
            Item::canvas(
                RectDraw::filled(rect, Color::rgb(r, g, b)).with_corner_radius(screen_radius),
            )
        }));
    }
    scene.push(carry.canvas(document, |rect| {
        Item::canvas(PageDraw {
            page: entity.id.clone(),
            rect,
            viewport: (frame.app.page_placement(&entity.id))
                .map_or_else(CssSize::default, |placement| placement.viewport),
            // The strips have the screen's corners, and the document's own
            // are square between them.
            corner_radius: if inside { 0.0 } else { screen_radius },
        })
    }));
    if !frame.chrome {
        return;
    }
    let zoom = frame.zoom();
    let on_screen = frame.project(outer);
    if let (Some(shell), Some((art, image))) = (&shell, art) {
        // The drawing goes over the page, so its island or notch covers it.
        // It has its own edge, so no ring is drawn round it.
        let (rect, turn) = art_rect(screen, shell, art);
        scene.push(carry.image(image, rect, turn));
    } else if let Some(shell) = &shell {
        if inside {
            trim_corners(device, &carry, screen, shell, scene);
        }
        decorate(device, &carry, screen, outer, shell, scene);
        let ring = Stroke::new(
            device.device_screen_ring,
            SCREEN_RING_WIDTH,
            StrokeAlign::Outside,
        );
        scene.push(carry.screen(frame.project(screen), |rect| {
            Item::screen(RectDraw::outlined(rect, ring).with_corner_radius(screen_radius * zoom))
        }));
    }
    if art.is_none() {
        // A ring just outside the frame, one pixel wide at any zoom.
        let border = Stroke::new(frame.colors.page_border, BORDER_WIDTH, StrokeAlign::Outside);
        scene.push(carry.screen(on_screen, |rect| {
            Item::screen(RectDraw::outlined(rect, border).with_corner_radius(outer_radius * zoom))
        }));
    }
}

/// The shell around `screen`, in canvas units.
pub(super) fn shell_rect(screen: Rect, shell: &DeviceShell) -> Rect {
    let insets = shell.insets;
    let (left, top) = (insets.left as f32, insets.top as f32);
    Rect::new(
        screen.x - left,
        screen.y - top,
        screen.width + left + insets.right as f32,
        screen.height + top + insets.bottom as f32,
    )
}

/// What of a device is under its page: its shadow, and the clay bezel or,
/// for a device that is `drawn`, the dark behind its drawing's screen.
fn under_page(
    device: &Colors,
    carry: &Carry,
    screen: Rect,
    shell: &DeviceShell,
    drawn: bool,
    scene: &mut Scene,
) {
    let outer = shell_rect(screen, shell);
    let radius = shell.corner_radius as f32;
    // The shadow falls down the screen whichever way the shell faces.
    scene.push(carry.canvas(outer, |rect| {
        Item::canvas(ShadowDraw {
            rect: Rect::new(rect.x, rect.y + SHELL_SHADOW, rect.width, rect.height),
            corner_radius: radius,
            blur: SHELL_SHADOW,
            color: device.device_shadow,
        })
    }));
    // The drawing's screen is a hole, so the dark behind the page is drawn
    // here.
    let (bezel, radius, color) = if drawn {
        (
            screen.outset(SCREEN_BACKING),
            shell.screen_corner_radius as f32 + SCREEN_BACKING,
            device.device_notch,
        )
    } else {
        (outer, radius, device.device_bezel)
    };
    scene.push(carry.canvas(bezel, |rect| {
        Item::canvas(RectDraw::filled(rect, color).with_corner_radius(radius))
    }));
}

/// The bezel again just outside `screen`, over a document that sits inside
/// its safe area. Where a strip is shallower than the screen's corner is
/// round, the document's square corner reaches past the curve, and a clay
/// bezel is under the page. A drawing is over it and needs none of this.
fn trim_corners(
    device: &Colors,
    carry: &Carry,
    screen: Rect,
    shell: &DeviceShell,
    scene: &mut Scene,
) {
    let insets = shell.insets;
    let bezel = [insets.top, insets.right, insets.bottom, insets.left]
        .into_iter()
        .fold(f64::INFINITY, f64::min) as f32;
    if bezel <= 0.0 {
        return;
    }
    let stroke = Stroke::new(device.device_bezel, bezel, StrokeAlign::Outside);
    let radius = shell.screen_corner_radius as f32;
    scene.push(carry.canvas(screen, |rect| {
        Item::canvas(RectDraw::outlined(rect, stroke).with_corner_radius(radius))
    }));
}

/// Where `art` is drawn round `screen`, and the turn it is drawn at. The
/// drawing stands up, scaled so its screen is the page's, and a page on its
/// side turns it a quarter turn clockwise about its centre, as the clay
/// shell's insets turn.
fn art_rect(screen: Rect, shell: &DeviceShell, art: DeviceArt) -> (Rect, f32) {
    let geometry = art.geometry();
    let upright_width = if shell.landscape {
        screen.height
    } else {
        screen.width
    };
    let scale = art.scale(f64::from(upright_width));
    let scaled = |units: f64| (units * scale) as f32;
    let (width, height) = (scaled(geometry.frame.0), scaled(geometry.frame.1));
    // From the screen's centre to the drawing's, standing up.
    let across = geometry.frame.0 / 2.0 - geometry.screen_origin.0 - geometry.screen_size.0 / 2.0;
    let down = geometry.frame.1 / 2.0 - geometry.screen_origin.1 - geometry.screen_size.1 / 2.0;
    let (across, down, turn) = if shell.landscape {
        (-scaled(down), scaled(across), std::f32::consts::FRAC_PI_2)
    } else {
        (scaled(across), scaled(down), 0.0)
    };
    let centre = screen.centre();
    let rect = Rect::new(
        centre.x + across - width / 2.0,
        centre.y + down - height / 2.0,
        width,
        height,
    );
    (rect, turn)
}

/// What a clay phone has on its front: a home button in the chin of one
/// with a square screen, and a notch on any other.
fn decorate(
    device: &Colors,
    carry: &Carry,
    screen: Rect,
    outer: Rect,
    shell: &DeviceShell,
    scene: &mut Scene,
) {
    if shell.kind != DeviceKind::Phone {
        return;
    }
    if shell.screen_corner_radius == 0.0 {
        home_button(device, carry, screen, shell, scene);
        return;
    }
    // Only a phone standing up has a notch, so it comes in with the turn.
    if !shell.landscape {
        let centre = outer.x + outer.width / 2.0;
        let (width, height, radius, gap) = NOTCH;
        let notch = Rect::new(centre - width / 2.0, screen.y + gap, width, height);
        let notch = carry.canvas(notch, |rect| {
            Item::canvas(RectDraw::filled(rect, device.device_notch).with_corner_radius(radius))
        });
        scene.push(notch.with_opacity(carry.progress));
    }
}

/// The home button, in the middle of the chin: under the screen of a phone
/// standing up, and left of it once the phone is on its side.
fn home_button(
    device: &Colors,
    carry: &Carry,
    screen: Rect,
    shell: &DeviceShell,
    scene: &mut Scene,
) {
    let (diameter, ring) = HOME_BUTTON;
    let middle = screen.centre();
    let (x, y) = if shell.landscape {
        (screen.x - shell.insets.left as f32 / 2.0, middle.y)
    } else {
        let chin = shell.insets.bottom as f32;
        (middle.x, screen.y + screen.height + chin / 2.0)
    };
    let button = Rect::new(x - diameter / 2.0, y - diameter / 2.0, diameter, diameter);
    let ring = Stroke::new(device.device_indicator, ring, StrokeAlign::Inside);
    scene.push(carry.canvas(button, |rect| {
        Item::canvas(RectDraw::outlined(rect, ring).with_corner_radius(diameter / 2.0))
    }));
}
