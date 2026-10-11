//! Pages: the live frame, and around it a border and a title.

use specular_core::CssSize;
use specular_doc::{DeviceArt, DeviceFrameColor, DeviceKind, DeviceShell, Entity, Page};
use specular_interact::{Appearance, Mark, PageState, title_scale};

use super::frame::{Frame, canvas_rect, carried};
use super::palette;
use crate::{
    Colors, ImageDraw, ImageId, Item, PageDraw, Point, Rect, RectDraw, Scene, ShadowDraw, Stroke,
    StrokeAlign, TextOverflow, TextRun, VerticalAlign,
};

/// Corner radius of a page's frame, in canvas units.
pub(crate) const CORNER_RADIUS: f32 = 8.0;
const BORDER_WIDTH: f32 = 1.0;
/// The title's line box and the gap under it, in logical pixels. A group's
/// title sits the same way above its rect.
pub(crate) const TITLE_LINE: f32 = specular_interact::TITLE_LINE;
pub(crate) const TITLE_GAP: f32 = specular_interact::TITLE_GAP;
pub(crate) const TITLE_SIZE: f32 = specular_interact::TITLE_SIZE;
pub(crate) const TITLE_WEIGHT: u16 = 500;

/// The drop and blur of a device frame's shadow, in canvas units.
const SHELL_SHADOW: f32 = 16.0;
/// The entered page's glow, in logical pixels: how far past the frame it is
/// at full strength, how far it fades from there, and how strong it is.
const ENTERED_GLOW_SPREAD: f32 = 2.0;
const ENTERED_GLOW_BLUR: f32 = 10.0;
const ENTERED_GLOW_ALPHA: f32 = 0.7;
/// The ring in the bezel around the screen, in logical pixels.
const SCREEN_RING_WIDTH: f32 = 1.5;
/// A phone's notch: its size, its corner radius and its gap from the top
/// of the screen, in canvas units.
const NOTCH: (f32, f32, f32, f32) = (126.0, 37.0, 18.5, 8.0);
/// The home indicator's height, in canvas units.
const INDICATOR_HEIGHT: f32 = 4.0;
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
    // The entered page glows in the selection colour from under its whole
    // frame, the same size on screen at any zoom.
    if frame.chrome && frame.app.mark(&entity.id) == Some(Mark::Entered) {
        let glow = frame.project(outer).outset(ENTERED_GLOW_SPREAD);
        scene.push(carry.screen(glow, |rect| {
            Item::screen(ShadowDraw {
                rect,
                corner_radius: outer_radius * frame.zoom() + ENTERED_GLOW_SPREAD,
                blur: ENTERED_GLOW_BLUR,
                color: palette::with_alpha(frame.colors.selection, ENTERED_GLOW_ALPHA),
            })
        }));
    }
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
    scene.push(carry.canvas(screen, |rect| {
        Item::canvas(PageDraw {
            page: entity.id.clone(),
            rect,
            viewport: (frame.app.page_placement(&entity.id))
                .map_or_else(CssSize::default, |placement| placement.viewport),
            corner_radius: screen_radius,
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
    if frame.app.names_in_dock(&entity.id) {
        return;
    }
    // The title stays upright and slides from where the shell was.
    let on_screen = specular_interact::turning_rect(frame.app, entity)
        .map_or(on_screen, |rect| frame.screen_rect(rect));
    let title = title(entity, page, frame.app.page_state(&entity.id));
    if !title.is_empty() {
        scene.push(title_above(
            frame,
            on_screen,
            &title,
            frame.colors.muted_text,
        ));
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

/// A phone's notch, and the home indicator in a phone's or a tablet's
/// bottom bezel.
fn decorate(
    device: &Colors,
    carry: &Carry,
    screen: Rect,
    outer: Rect,
    shell: &DeviceShell,
    scene: &mut Scene,
) {
    let (phone, tablet) = match shell.kind {
        DeviceKind::Phone => (true, false),
        DeviceKind::Tablet => (false, true),
        DeviceKind::Plain => return,
    };
    let centre = outer.x + outer.width / 2.0;
    if phone && shell.screen_corner_radius > 0.0 && !shell.landscape {
        let (width, height, radius, gap) = NOTCH;
        let notch = Rect::new(centre - width / 2.0, screen.y + gap, width, height);
        // Only a phone standing up has a notch, so it comes in with the turn.
        let notch = carry.canvas(notch, |rect| {
            Item::canvas(RectDraw::filled(rect, device.device_notch).with_corner_radius(radius))
        });
        scene.push(notch.with_opacity(carry.progress));
    }
    let width = if phone && !shell.landscape {
        120.0
    } else {
        100.0
    };
    let bezel = shell.insets.bottom as f32;
    let lift = (bezel / 2.0 - if tablet { 3.0 } else { 4.0 }).max(4.0);
    let indicator = Rect::new(
        centre - width / 2.0,
        outer.y + outer.height - lift - INDICATOR_HEIGHT,
        width,
        INDICATOR_HEIGHT,
    );
    scene.push(carry.canvas(indicator, |rect| {
        Item::canvas(
            RectDraw::filled(rect, device.device_indicator)
                .with_corner_radius(INDICATOR_HEIGHT / 2.0),
        )
    }));
}

/// What the title line of a page says. A label the user gave wins. Without
/// one it reads `Title — address` once the page has a title, and the address
/// alone before that; while a load is in flight it starts `Loading… `.
fn title(entity: &Entity, page: &Page, state: Option<&PageState>) -> String {
    let label = entity.label.as_deref().filter(|label| !label.is_empty());
    let loading = state.is_some_and(|state| state.loading);
    let line = if let Some(label) = label {
        label.to_owned()
    } else {
        let url = state.and_then(|state| state.url.as_deref());
        let address = address(url.unwrap_or(&page.url));
        match state
            .map(|state| state.title.trim())
            .filter(|title| !title.is_empty())
        {
            Some(title) => format!("{title} \u{2014} {address}"),
            None => address.to_owned(),
        }
    };
    if loading && !line.is_empty() {
        format!("Loading\u{2026} {line}")
    } else {
        line
    }
}

/// An address without its scheme or trailing slash.
fn address(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.trim_end_matches('/')
}

/// One line of chrome text above the top-left corner of `on_screen`. It is
/// never wider than the rect: a longer title ends in an ellipsis. It keeps
/// its pixel size down to half zoom and shrinks with the canvas below that,
/// so zoomed out it does not outgrow what it names or run into its
/// neighbours. Small enough, the renderer stops drawing it.
pub(crate) fn title_above(
    frame: &Frame<'_>,
    on_screen: Rect,
    text: &str,
    color: crate::Color,
) -> Item {
    let scale = title_scale(frame.zoom());
    let (line, gap) = (TITLE_LINE * scale, TITLE_GAP * scale);
    let top = on_screen.y - gap - line;
    let run = TextRun {
        wrap_width: Some(on_screen.width),
        overflow: TextOverflow::Ellipsis,
        box_height: Some(line),
        weight: TITLE_WEIGHT,
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(
            text,
            crate::Point::new(on_screen.x, top),
            TITLE_SIZE * scale,
            color,
        )
    };
    Item::screen(run).clipped(Rect::new(on_screen.x, top, on_screen.width, line + gap))
}
