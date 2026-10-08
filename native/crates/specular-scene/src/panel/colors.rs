//! The colors of the built-in panels: the light theme of
//! `src/renderer/shared/surfaceTheme.css`, with Tailwind's stone and zinc
//! scales resolved to sRGB.

use crate::Color;

/// The toolbar strip: `--surface-toolbar`, stone-300.
pub(super) const TOOLBAR: Color = Color::rgb(0xd6, 0xd3, 0xd1);
/// The line under the toolbar: `--surface-toolbar-border`, stone-400.
pub(super) const TOOLBAR_BORDER: Color = crate::view::palette::PAGE_BORDER;
/// A tool button that is hovered or active: `#fdf8f5`.
pub(super) const TOOL_FILL: Color = crate::view::palette::NEUTRAL_FILL;
/// Toolbar text at rest and when hovered: zinc-600 and zinc-900.
pub(super) const TOOLBAR_TEXT: Color = Color::rgb(0x52, 0x52, 0x5c);
pub(super) const TOOLBAR_TEXT_STRONG: Color = Color::rgb(0x18, 0x18, 0x1b);
/// The chevron beside the zoom readout: `#45403C` at 40%.
pub(super) const TOOLBAR_CHEVRON: Color = Color::rgba(0x45, 0x40, 0x3c, 102);

/// A floating panel: `--surface-popup`, stone-50.
pub(super) const POPUP: Color = crate::view::palette::CARD;
/// Its border: `--surface-popup-border`, stone-200 and stone-400 mixed
/// 55 to 45.
pub(super) const POPUP_BORDER: Color = Color::rgb(0xca, 0xc6, 0xc3);
/// The two shadows of `popupSurfaceStyle`: `0 10px 8px -6px
/// rgba(0,0,0,.18)` and `0 4px 16px 0 rgba(199,193,188,.5)`.
pub(super) const SHADOW_NEAR: Color = Color::rgba(0, 0, 0, 46);
pub(super) const SHADOW_FAR: Color = Color::rgba(199, 193, 188, 128);

/// A menu of words: `bg-white` with `border-zinc-200`, and the two layers
/// of `shadow-xl`, each `rgb(0 0 0 / 0.1)`.
pub(super) const MENU: Color = Color::rgb(0xff, 0xff, 0xff);
pub(super) const MENU_BORDER: Color = Color::rgb(0xe4, 0xe4, 0xe7);
pub(super) const MENU_SHADOW: Color = Color::rgba(0, 0, 0, 26);

/// Panel text: `--surface-foreground`, and the same at 58% for
/// `--surface-foreground-muted`.
pub(super) const TEXT: Color = Color::rgb(0x30, 0x30, 0x30);
pub(super) const TEXT_MUTED: Color = Color::rgba(0x30, 0x30, 0x30, 148);
/// A hovered control and one that is on: stone-100 and stone-200.
pub(super) const HOVER: Color = Color::rgb(0xf5, 0xf5, 0xf4);
pub(super) const ON: Color = Color::rgb(0xe7, 0xe5, 0xe4);
/// The highlighted row of a menu: zinc-100.
pub(super) const MENU_HOVER: Color = Color::rgb(0xf4, 0xf4, 0xf5);
/// A divider between groups: zinc-900 at 20%.
pub(super) const DIVIDER: Color = Color::rgba(0x18, 0x18, 0x1b, 51);
/// A line between sections of a list: zinc-200.
pub(super) const RULE: Color = Color::rgb(0xe4, 0xe4, 0xe7);
/// The outline of a field: zinc-300.
pub(super) const FIELD_BORDER: Color = Color::rgb(0xd4, 0xd4, 0xd8);
/// A text field's fill: `bg-white`.
pub(super) const INPUT: Color = Color::rgb(0xff, 0xff, 0xff);
/// The ring of a text field that has the keys: `ring-blue-500/40`.
pub(super) const INPUT_RING: Color = Color::rgba(0x3b, 0x82, 0xf6, 102);
/// A key hint: stone-600 on stone-200.
pub(super) const KEY: Color = ON;
pub(super) const KEY_TEXT: Color = Color::rgb(0x57, 0x53, 0x4d);

/// The hairline around a swatch's dot: `rgba(0,0,0,0.12)`.
pub(super) const DOT_EDGE: Color = Color::rgba(0, 0, 0, 31);
/// The ring of a selected swatch too pale to ring itself: zinc-500.
pub(super) const RING_GRAY: Color = Color::rgb(0x71, 0x71, 0x7a);
/// A control that cannot be used: `disabled:opacity-30`.
pub(super) const DISABLED: f32 = 0.3;

/// The sidebar's ground: `--surface-panel`.
pub(super) const SIDEBAR: Color = Color::rgb(0xf2, 0xf2, 0xf2);
/// A line inside the sidebar: `--surface-panel-border`, stone-200.
pub(super) const SIDEBAR_RULE: Color = Color::rgb(0xe7, 0xe5, 0xe4);
/// A selected row, and a pressed button: `--surface-interactive`, stone-200
/// at 80%.
pub(super) const INTERACTIVE: Color = Color::rgba(0xe7, 0xe5, 0xe4, 204);
/// A hovered row or button: `--surface-interactive-hover`, stone-200 at 40%.
pub(super) const INTERACTIVE_HOVER: Color = Color::rgba(0xe7, 0xe5, 0xe4, 102);
/// The thumb of a scrollbar: `rgba(0, 0, 0, 0.2)`.
pub(super) const SCROLL_THUMB: Color = Color::rgba(0, 0, 0, 51);
/// A row faded because its page has left its document: `opacity-50`.
pub(super) const DIMMED: f32 = 0.5;
