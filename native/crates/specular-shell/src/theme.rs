//! The Electron app's light theme in the Kit's theme, so the Kit's
//! components come out in Specular's colours and not the Kit's defaults.
//!
//! Every value is from `src/renderer/shared/surfaceTheme.css` and the
//! Tailwind classes of the toolbar, the left sidebar and the popups, with
//! the stone and zinc scales resolved to sRGB.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Hsla, Rgba, px, rgb, rgba};

/// `--surface-panel`: the sidebar and a dialog.
pub(crate) const PANEL: u32 = 0x00f2_f2f2;
/// `--surface-foreground`.
pub(crate) const TEXT: u32 = 0x0030_3030;
/// `--surface-foreground-muted`: the foreground at 58%.
pub(crate) const TEXT_MUTED: u32 = 0x3030_3094;
/// `--surface-chrome-border`: the sidebar's edge and a popup's.
pub(crate) const CHROME_BORDER: u32 = 0x00ca_c6c3;
/// `--surface-panel-border`, stone-200.
pub(crate) const PANEL_BORDER: u32 = 0x00e7_e5e4;
/// `--surface-toolbar`, stone-300.
pub(crate) const TOOLBAR: u32 = 0x00d6_d3d1;
/// `--surface-toolbar-border`, stone-400.
pub(crate) const TOOLBAR_BORDER: u32 = 0x00a6_a09b;
/// Toolbar text at rest and when hovered: zinc-600 and zinc-900.
pub(crate) const TOOLBAR_TEXT: u32 = 0x0052_525c;
pub(crate) const TOOLBAR_TEXT_STRONG: u32 = 0x0018_181b;
/// A tool button that is hovered or active.
pub(crate) const TOOL_FILL: u32 = 0x00fd_f8f5;
/// `--surface-interactive-hover` and `--surface-interactive`: stone-200 at
/// 40% and at 80%. A hovered row, and a selected one.
pub(crate) const ROW_HOVER: u32 = 0xe7e5_e466;
pub(crate) const ROW_SELECTED: u32 = 0xe7e5_e4cc;
/// `--surface-popup`, stone-50: a floating panel.
pub(crate) const POPUP: u32 = 0x00fa_faf9;
/// A hovered popup control and one that is on: stone-100 and stone-200.
pub(crate) const CONTROL_HOVER: u32 = 0x00f5_f5f4;
pub(crate) const CONTROL_ON: u32 = 0x00e7_e5e4;
/// A divider between groups of a popup: zinc-900 at 20%.
pub(crate) const DIVIDER: u32 = 0x1818_1b33;
/// The hairline around a swatch's dot: black at 12%.
pub(crate) const DOT_EDGE: u32 = 0x0000_001f;
/// The ring of a selected swatch too pale to ring itself: zinc-500.
pub(crate) const RING_GRAY: u32 = 0x0071_717a;
/// `--surface-focus-ring`, blue-500.
pub(crate) const FOCUS_RING: u32 = 0x002b_7fff;
/// `--surface-primary` and its hover: stone-900 and stone-700.
const PRIMARY: u32 = 0x001c_1917;
const PRIMARY_HOVER: u32 = 0x0044_403b;
/// `--surface-input-border`, stone-300.
const INPUT_BORDER: u32 = 0x00d6_d3d1;

/// The height of the toolbar strip, `TOOLBAR_HEIGHT` in
/// `src/shared/constants.ts`. The app's own layout assumes it too.
pub(crate) const TOOLBAR_HEIGHT: f32 = specular_interact::panel::builtin::TOOLBAR_HEIGHT;
/// The sidebar's width, `LEFT_SIDEBAR_WIDTH` in `runtime-constants.ts`.
pub(crate) const SIDEBAR_WIDTH: f32 = 256.0;

/// An opaque `0xRRGGBB`.
pub(crate) fn solid(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// A see-through `0xRRGGBBAA`.
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

/// Puts the light theme in place. Call after `gpui_kit::init`, which loads
/// the Kit's own.
pub(crate) fn apply(cx: &mut App) {
    // Changing the mode loads the Kit's colours, so ours go on afterwards.
    Theme::change(ThemeMode::Light, None, cx);
    Theme::update(cx, |theme| {
        theme.radius = px(6.0);
        theme.radius_lg = px(10.0);
        theme.shadow = true;

        theme.background = solid(PANEL);
        theme.foreground = solid(TEXT);
        theme.muted = solid(CONTROL_HOVER);
        theme.muted_foreground = tinted(TEXT_MUTED);
        theme.border = solid(CHROME_BORDER);
        theme.input = solid(INPUT_BORDER);
        theme.ring = solid(FOCUS_RING);
        theme.caret = solid(TEXT);
        theme.selection = solid(0x00b3_d7ff);

        theme.primary = solid(PRIMARY);
        theme.primary_hover = solid(PRIMARY_HOVER);
        theme.primary_active = solid(0x0029_2524);
        theme.primary_foreground = solid(POPUP);
        theme.secondary = solid(PANEL);
        theme.secondary_hover = solid(CONTROL_ON);
        theme.secondary_active = solid(CONTROL_ON);
        theme.secondary_foreground = solid(TEXT);
        theme.accent = solid(CONTROL_ON);
        theme.accent_foreground = solid(TEXT);

        theme.popover = solid(POPUP);
        theme.popover_foreground = solid(TEXT);
        theme.colors.list = solid(PANEL);
        theme.list_hover = tinted(ROW_HOVER);
        theme.list_active = tinted(ROW_SELECTED);
        theme.list_active_border = tinted(ROW_SELECTED);

        theme.sidebar = solid(PANEL);
        theme.sidebar_foreground = solid(TEXT);
        theme.sidebar_border = solid(CHROME_BORDER);
        theme.sidebar_accent = tinted(ROW_SELECTED);
        theme.sidebar_accent_foreground = solid(TEXT);
        theme.sidebar_primary = solid(PRIMARY);
        theme.sidebar_primary_foreground = solid(POPUP);

        theme.title_bar = solid(TOOLBAR);
        theme.title_bar_border = solid(TOOLBAR_BORDER);
        theme.switch = solid(INPUT_BORDER);
        theme.switch_thumb = solid(0x00ff_ffff);
        theme.scrollbar_thumb = tinted(0x0000_0033);
        theme.scrollbar_thumb_hover = tinted(0x0000_004d);
        theme.drop_target = tinted(0x2b7f_ff1a);
    });
}
