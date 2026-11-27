//! The Electron app's themes in the Kit's theme, so the Kit's components come
//! out in Specular's colours and not the Kit's defaults.
//!
//! The chrome colours are `specular_scene::PanelColors`, which the canvas
//! reads too. The few only the Kit draws (the right panel's zinc scale,
//! the primary button) are here, light and dark, from `surfaceTheme.css` and the Tailwind classes of the right panel.
//! The colours are functions of the theme in force, which
//! [`apply`] sets.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Hsla, Pixels, Rgba, WindowAppearance, px, rgba};
use specular_interact::{Appearance, Theme as Choice};
use specular_scene::{Color, Colors};

/// Whether the dark theme is in force.
static DARK: AtomicBool = AtomicBool::new(false);

/// What GPUI says the operating system looks like.
pub(crate) const fn of_system(appearance: WindowAppearance) -> Appearance {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
    }
}

/// What the app is told the system looks like at start. A scripted run is
/// told light, as a headless one is, and its `appearance` steps are the
/// system from then on, so the same script means the same in both.
pub(crate) fn system_or_script(appearance: WindowAppearance) -> Appearance {
    if crate::debug_input::scripted() {
        Appearance::Light
    } else {
        of_system(appearance)
    }
}

thread_local! {
    /// The choice `AppKit` was last told of.
    static CHOICE: Cell<Choice> = const { Cell::new(Choice::System) };
}

/// Whether the app draws as the system does.
pub(crate) fn follows_system() -> bool {
    CHOICE.get() == Choice::System
}

/// Tells `AppKit` of the choice `theme`, so the parts of the window it
/// draws match ours. Returns what the system looks like when the choice
/// became the system's: while another was in force the window said nothing
/// of it.
pub(crate) fn choose(theme: Choice) -> Option<Appearance> {
    if CHOICE.replace(theme) == theme {
        return None;
    }
    crate::native::set_app_appearance(match theme {
        Choice::System => None,
        Choice::Light => Some(false),
        Choice::Dark => Some(true),
    });
    (theme == Choice::System && !crate::debug_input::scripted()).then(|| {
        if crate::native::system_is_dark() {
            Appearance::Dark
        } else {
            Appearance::Light
        }
    })
}

/// The theme in force.
pub(crate) fn appearance() -> Appearance {
    if DARK.load(Ordering::Relaxed) {
        Appearance::Dark
    } else {
        Appearance::Light
    }
}

fn colors() -> &'static Colors {
    Colors::of(appearance())
}

/// `color` as `0xRRGGBBAA`, which is what every colour here is.
fn packed(color: Color) -> u32 {
    u32::from(color.r) << 24
        | u32::from(color.g) << 16
        | u32::from(color.b) << 8
        | u32::from(color.a)
}

/// One colour of the set in force, light or dark.
fn pick(light: u32, dark: u32) -> u32 {
    match appearance() {
        Appearance::Light => light,
        Appearance::Dark => dark,
    }
}

/// [`pick`] for two colours that go together.
fn pick_pair(light: (u32, u32), dark: (u32, u32)) -> (u32, u32) {
    match appearance() {
        Appearance::Light => light,
        Appearance::Dark => dark,
    }
}

macro_rules! panel_colors {
    ($($(#[$doc:meta])* $name:ident => $field:ident, $key:literal;)*) => {
        $(
            $(#[$doc])*
            pub(crate) fn $name() -> u32 {
                crate::look::over($key).unwrap_or_else(|| packed(colors().panel.$field))
            }
        )*

        /// A panel colour by the name a mod's `colors()` gives it.
        pub(crate) fn by_name(name: &str) -> Option<u32> {
            match name {
                $($key => Some($name()),)*
                "raisedFill" => Some(raised_fill()),
                "tabHover" => Some(tab_hover()),
                "primary" => Some(primary()),
                "primaryHover" => Some(primary_hover()),
                "primaryForeground" => Some(primary_foreground()),
                "error" => Some(error()),
                "focusRing" => Some(focus_ring()),
                "hoverText" => Some(hover_text()),
                "onText" => Some(on_text()),
                "accent" => Some(crate::look::over("accent").unwrap_or_else(focus_ring)),
                _ => None,
            }
        }
    };
}

panel_colors! {
    /// `--surface-panel`: the sidebar and a dialog.
    panel => sidebar, "sidebar";
    /// `--surface-foreground`.
    text => text, "text";
    /// `--surface-foreground-muted`.
    text_muted => text_muted, "textMuted";
    /// `--surface-chrome-border`: the sidebar's edge and a popup's.
    chrome_border => popup_border, "popupBorder";
    /// `--surface-panel-border`.
    panel_border => sidebar_rule, "sidebarRule";
    /// `--surface-toolbar`.
    toolbar => toolbar, "toolbar";
    /// `--surface-toolbar-border`.
    toolbar_border => toolbar_border, "toolbarBorder";
    /// Toolbar text at rest.
    toolbar_text => toolbar_text, "toolbarText";
    /// Toolbar text when hovered or on.
    toolbar_text_strong => toolbar_text_strong, "toolbarTextStrong";
    /// `--surface-interactive-hover`: a hovered row.
    row_hover => interactive_hover, "interactiveHover";
    /// `--surface-interactive`: a selected row.
    row_selected => interactive, "interactive";
    /// `--surface-popup`: a floating panel.
    popup => popup, "popup";
    /// A hovered popup control.
    control_hover => hover, "hover";
    /// A popup control that is on.
    control_on => on, "on";
    /// A divider between groups of a popup.
    divider => divider, "divider";
    /// A key hint's text.
    key_text => key_text, "keyText";
    /// The hairline around a swatch's dot.
    dot_edge => dot_edge, "dotEdge";
    /// The ring of a selected swatch too pale to ring itself.
    ring_gray => ring_gray, "ringGray";
}

/// The fill of a raised control (`view/raised.rs`): the tool fill at 60%
/// over the toolbar in the light theme and 9% in the dark, as one opaque
/// colour, so the shadow under it does not show through and a label has one
/// colour to fade into. The rim and the shadow carry the rest of the lift.
pub(crate) fn raised_fill() -> u32 {
    crate::look::over("raisedFill").unwrap_or_else(|| pick(0xede9_e7ff, 0x5450_4bff))
}

/// A hovered tab that is not showing, opaque over the tab row.
pub(crate) fn tab_hover() -> u32 {
    crate::look::over("tabHover").unwrap_or_else(|| pick(0xdfdc_daff, 0x4b47_42ff))
}

/// The rim of a raised control, from its top to its bottom: lit above and
/// fainter below.
pub(crate) fn raised_rim() -> (u32, u32) {
    pick_pair((0xffff_ffcc, 0x0000_0014), (0xffff_ff26, 0xffff_ff0d))
}

/// The two shadows under a raised control, the nearer first.
pub(crate) fn raised_shadow() -> (u32, u32) {
    pick_pair((0x0000_001a, 0x0000_000d), (0x0000_0040, 0x0000_0026))
}

/// The shade a raised control's face takes into its bottom edge.
pub(crate) fn raised_shade() -> u32 {
    pick(0x0000_000a, 0x0000_0033)
}

/// `--surface-focus-ring`: blue-500, blue-400 in the dark, unless the
/// layout names a `focusRing`.
pub(crate) fn focus_ring() -> u32 {
    crate::look::over("focusRing").unwrap_or_else(|| pick(0x2b7f_ffff, 0x51a2_ffff))
}

/// The text of a hovered row or control: the text colour, unless the theme
/// names a `hoverText` for a `hover` too dark or too light for it.
pub(crate) fn hover_text() -> u32 {
    crate::look::over("hoverText").unwrap_or_else(text)
}

/// The text of a row or control that is on, over the `on` fill: the text
/// colour, unless the theme names an `onText`.
pub(crate) fn on_text() -> u32 {
    crate::look::over("onText").unwrap_or_else(text)
}

/// `--surface-primary`: stone-900, stone-100 in the dark.
pub(crate) fn primary() -> u32 {
    crate::look::over("primary").unwrap_or_else(|| pick(0x1c19_17ff, 0xf5f5_f4ff))
}

/// `--surface-primary-hover`: stone-700, stone-300.
pub(crate) fn primary_hover() -> u32 {
    crate::look::over("primaryHover").unwrap_or_else(|| pick(0x4440_3bff, 0xd6d3_d1ff))
}

/// `--surface-primary` pressed.
fn primary_active() -> u32 {
    pick(0x2925_24ff, 0xa6a0_9bff)
}

/// `--surface-primary-foreground`: stone-50, stone-900.
pub(crate) fn primary_foreground() -> u32 {
    crate::look::over("primaryForeground").unwrap_or_else(|| pick(0xfafa_f9ff, 0x1c19_17ff))
}

/// The highlight on selected text. The Kit paints it over the glyphs, so it
/// is the scene's `text_selection` hue as a tint, at the 30% alpha the Kit
/// caps its own themes to.
fn text_selection() -> u32 {
    pick(0x419b_ff4d, 0x62ad_ff4d)
}

/// `--surface-input`: stone-50 in the light, a step lighter than the panel
/// in the dark. A sent message's bubble and the run bar.
pub(crate) fn input() -> u32 {
    // pick(0xfafa_f9ff, 0x1d1a_18ff)
    pick(0xfafa_f9ff, 0x332f_2dff)
}

/// `--surface-input-border`: stone-300, and a neutral grey in the dark.
pub(crate) fn input_border() -> u32 {
    // pick(0xd6d3_d1ff, 0x3f3b_36ff)
    pick(0xd6d3_d1ff, 0x3f3f_3fff)
}

/// The muted foreground as an opaque colour over the panel, for a glyph.
pub(crate) fn glyph_muted() -> u32 {
    pick(0x8181_81ff, 0xa7a5_a6ff)
}

/// Over the window behind a dialog: a dimming the window shows through.
pub(crate) fn scrim() -> u32 {
    pick(0x0000_0024, 0x0000_0066)
}

/// Behind the first-run view: a step darker than the panel in light, the
/// panel itself in dark.
pub(crate) fn backdrop() -> u32 {
    pick(0xe4e4_e4ff, panel())
}

/// The zinc scale the right panel's composer is built from: a chip and a
/// hovered row (100) and its edge (300). The dark theme takes the opposite
/// end of the scale.
pub(crate) fn zinc_100() -> u32 {
    pick(0xf4f4_f5ff, 0x2727_2aff)
}
pub(crate) fn zinc_300() -> u32 {
    pick(0xd4d4_d8ff, 0x5252_5cff)
}

/// A queued message's chip: zinc-200 at 60%.
pub(crate) fn queued() -> u32 {
    pick(0xe4e4_e799, 0x3f3f_4799)
}

/// A hovered composer chip: zinc-200 at 70%.
pub(crate) fn chip_hover() -> u32 {
    pick(0xe4e4_e7b3, 0x3f3f_47b3)
}

/// The run bar's wash: the ends of `GrainGradient`'s palette under its veil,
/// white at 45% in the light theme and black at 45% in the dark. The grain
/// and the drift are a shader this renderer does not have.
pub(crate) fn wash_from() -> u32 {
    pick(0xf0d9_ffff, 0x7d6c_8cff)
}
pub(crate) fn wash_to() -> u32 {
    pick(0xffe2_c2ff, 0x8c6f_4fff)
}

/// The run bar's label: black at 55%, white at 60% in the dark.
pub(crate) fn label() -> u32 {
    pick(0x0000_008c, 0xffff_ff99)
}

/// A failed run's words: red-600, red-400 in the dark.
pub(crate) fn error() -> u32 {
    crate::look::over("error").unwrap_or_else(|| pick(0xe700_0bff, 0xff64_67ff))
}

/// The chrome's one row, and the bar under it that holds the dock, which
/// the app's own layout assumes too. Everything under the chrome starts at
/// [`chrome_top`], which is `CHROME_HEIGHT` while the tab row is drawn.
pub(crate) use specular_interact::panel::{CHROME_HEIGHT, DOCK_ROW, TAB_ROW};

thread_local! {
    /// Where the regions under the tab row start: under it, or at the
    /// window's top while the layout hides it.
    static CHROME_TOP: Cell<f32> = const { Cell::new(CHROME_HEIGHT) };
}

/// Where the regions under the tab row start, as the last render set it.
pub(crate) fn chrome_top() -> f32 {
    CHROME_TOP.get()
}

/// Sets where the regions under the tab row start, for this render.
pub(crate) fn set_chrome_top(top: f32) {
    CHROME_TOP.set(top);
}

/// `0xRRGGBBAA` as a GPUI colour.
pub(crate) fn solid(hex: u32) -> Hsla {
    rgba(hex).into()
}

/// `0xRRGGBBAA` as a GPUI colour: the same as [`solid`], named for the call
/// sites whose colour lets what is under it show.
pub(crate) fn tinted(hex: u32) -> Hsla {
    rgba(hex).into()
}

/// A scene colour, as the canvas's own pass would paint it.
pub(crate) fn of_scene(color: specular_scene::Color) -> Hsla {
    Rgba {
        r: f32::from(color.r) / 255.0,
        g: f32::from(color.g) / 255.0,
        b: f32::from(color.b) / 255.0,
        a: f32::from(color.a) / 255.0,
    }
    .into()
}

/// Gives the Kit's text system the bundled fonts the canvas draws with, so
/// the panels and the canvas are set in the same faces.
pub(crate) fn load_fonts(cx: &App) {
    let fonts = specular_compositor::bundled_fonts()
        .iter()
        .map(|&font| std::borrow::Cow::Borrowed(font))
        .collect();
    if let Err(error) = cx.text_system().add_fonts(fonts) {
        tracing::warn!("loading the bundled fonts: {error:#}");
    }
}

/// `points`, a corner radius drawn at the app's own 6 point radius, scaled
/// to the layout's or theme's `radius`: the same at the default, square at
/// 0, so every corner of the app's own panels follows one setting.
pub(crate) fn corner_points(points: f32) -> f32 {
    points * crate::look::radius() / 6.0
}

/// [`corner_points`] as a length.
pub(crate) fn corner(points: f32) -> Pixels {
    px(corner_points(points))
}

/// A pill's or a circle's corners, unless the look squares every corner.
pub(crate) fn corner_full() -> Pixels {
    if crate::look::radius() > 0.0 {
        px(9999.0)
    } else {
        px(0.0)
    }
}

/// Puts the theme of `appearance` in place. Call after `gpui_kit::init`,
/// which loads the Kit's own, and again whenever the theme changes.
pub(crate) fn apply(appearance: Appearance, cx: &mut App) {
    DARK.store(appearance == Appearance::Dark, Ordering::Relaxed);
    let mode = match appearance {
        Appearance::Light => ThemeMode::Light,
        Appearance::Dark => ThemeMode::Dark,
    };
    // Changing the mode loads the Kit's colours, so ours go on afterwards.
    Theme::change(mode, None, cx);
    Theme::update(cx, |theme| {
        theme.font_family = (crate::look::font())
            .map_or_else(|| specular_compositor::SANS_FAMILY.into(), Into::into);
        theme.mono_font_family = specular_compositor::MONO_FAMILY.into();
        theme.radius = px(crate::look::radius());
        theme.radius_lg = px(crate::look::radius_large());
        theme.shadow = true;

        theme.background = solid(panel());
        theme.foreground = solid(text());
        theme.muted = solid(control_hover());
        theme.muted_foreground = tinted(text_muted());
        theme.border = solid(chrome_border());
        theme.input = solid(input_border());
        theme.ring = solid(focus_ring());
        theme.caret = solid(text());
        theme.selection = tinted(text_selection());

        theme.primary = solid(primary());
        theme.primary_hover = solid(primary_hover());
        theme.primary_active = solid(primary_active());
        theme.primary_foreground = solid(primary_foreground());
        theme.secondary = solid(panel());
        theme.secondary_hover = solid(control_on());
        theme.secondary_active = solid(control_on());
        theme.secondary_foreground = solid(text());
        theme.accent = solid(control_on());
        theme.accent_foreground = solid(on_text());

        theme.popover = solid(popup());
        theme.popover_foreground = solid(text());
        theme.colors.list = solid(panel());
        theme.list_hover = tinted(row_hover());
        theme.list_active = tinted(row_selected());
        theme.list_active_border = tinted(row_selected());

        theme.sidebar = solid(panel());
        theme.sidebar_foreground = solid(text());
        theme.sidebar_border = solid(chrome_border());
        theme.sidebar_accent = tinted(row_selected());
        theme.sidebar_accent_foreground = solid(text());
        theme.sidebar_primary = solid(primary());
        theme.sidebar_primary_foreground = solid(primary_foreground());

        theme.title_bar = solid(toolbar());
        theme.title_bar_border = solid(toolbar_border());
        theme.switch = solid(input_border());
        theme.switch_thumb = solid(pick(0xffff_ffff, 0xf4f4_f5ff));
        theme.scrollbar_thumb = tinted(packed(colors().panel.scroll_thumb));
        theme.scrollbar_thumb_hover = tinted(pick(0x0000_004d, 0xffff_ff4d));
        theme.drop_target = tinted(0x2b7f_ff1a);
    });
}
