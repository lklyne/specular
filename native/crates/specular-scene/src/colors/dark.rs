//! The dark theme: the `.dark` block of `surfaceTheme.css` and the `isDark`
//! branches of the canvas renderers.

use super::{Colors, Hues, PanelColors, Shade};
use specular_interact::Appearance;

use crate::{Blend, Color};

pub(super) const COLORS: Colors = Colors {
    appearance: Appearance::Dark,
    // `--surface-canvas`, stone-800 at 60%, over the window's `#44403c`.
    canvas: Color::rgb(0x34, 0x30, 0x2e),
    dot: Color::rgb(0x78, 0x71, 0x6c),
    dot_floor: 0.56,
    ink: Color::rgb(0xe7, 0xe5, 0xe4),
    // A sticky keeps dark text: its dark-mode paper is still light.
    paper_ink: Color::rgb(0x1c, 0x19, 0x17),
    neutral_fill: Color::rgb(0xdc, 0xd2, 0xc4),
    // The saturated hues, undimmed (`--canvas-hue-*`).
    hues: Hues {
        red: Color::rgb(0xff, 0x10, 0x16),
        orange: Color::rgb(0xff, 0x8e, 0x00),
        yellow: Color::rgb(0xff, 0xd5, 0x00),
        green: Color::rgb(0x00, 0xca, 0x48),
        cyan: Color::rgb(0x00, 0xcb, 0xff),
        purple: Color::rgb(0xbd, 0x4b, 0xe5),
        blue: Color::rgb(0x10, 0x84, 0xff),
    },
    selection: Color::rgb(0x60, 0xa5, 0xfa),
    text_selection: Color::rgb(0x3f, 0x63, 0x8b),
    page_border: Color::rgb(0x57, 0x53, 0x4d),
    device_bezel: Color::rgb(0x29, 0x25, 0x24),
    device_shadow: Color::rgba(0, 0, 0, 204),
    device_notch: Color::rgb(0, 0, 0),
    device_indicator: Color::rgba(255, 255, 255, 64),
    device_screen_ring: Color::rgba(255, 255, 255, 15),
    muted_text: Color::rgb(0xa6, 0xa0, 0x9b),
    card: Color::rgb(0x1c, 0x19, 0x17),
    card_shadow: 77,
    file_glyph: Color::rgb(0xa8, 0xa2, 0x9e),
    link: Color::rgb(0x60, 0xa5, 0xfa),
    group_fill: Color::rgba(39, 39, 42, 89),
    group_border: Color::rgba(161, 161, 170, 64),
    group_title: Color::rgb(0xd4, 0xd4, 0xd8),
    group_tinted_title: Color::rgb(0xf4, 0xf4, 0xf5),
    group_tint: 0.2,
    group_border_mix: (Color::rgb(0xf4, 0xf4, 0xf5), 0.72),
    shape_fill: Shade::Darken(0.55),
    shape_border: Shade::Keep,
    shape_label: Color::rgb(220, 220, 220),
    highlight: Blend::Normal,
    composer: Color::rgb(0x18, 0x18, 0x1b),
    composer_border: Color::rgb(0x52, 0x52, 0x5c),
    composer_ink: Color::rgb(0xf4, 0xf4, 0xf5),
    composer_hint: Color::rgb(0x9f, 0x9f, 0xa9),
    panel: PanelColors {
        toolbar: Color::rgb(0x44, 0x40, 0x3b),
        toolbar_border: Color::rgb(0x57, 0x53, 0x4d),
        tool_fill: Color::rgba(253, 248, 245, 26),
        toolbar_text: Color::rgb(0xd4, 0xd4, 0xd8),
        toolbar_text_strong: Color::rgb(0xf4, 0xf4, 0xf5),
        toolbar_chevron: Color::rgba(0xe2, 0xde, 0xdb, 102),
        // `--surface-popup`: the panel and stone-700, 45 to 55.
        popup: Color::rgb(0x38, 0x34, 0x31),
        popup_border: Color::rgb(0x4d, 0x49, 0x43),
        shadow_near: Color::rgba(0, 0, 0, 148),
        shadow_far: Color::rgba(0, 0, 0, 128),
        menu: Color::rgb(0x18, 0x18, 0x1b),
        menu_border: Color::rgb(0x3f, 0x3f, 0x47),
        menu_shadow: Color::rgba(0, 0, 0, 26),
        text: Color::rgb(0xf4, 0xf4, 0xf5),
        text_muted: Color::rgba(0xf4, 0xf4, 0xf5, 158),
        hover: Color::rgba(253, 248, 245, 26),
        on: Color::rgba(253, 248, 245, 26),
        menu_hover: Color::rgb(0x27, 0x27, 0x2a),
        divider: Color::rgba(255, 255, 255, 51),
        rule: Color::rgb(0x3f, 0x3f, 0x47),
        field_border: Color::rgba(0x44, 0x40, 0x3b, 204),
        input: Color::rgba(0x1c, 0x19, 0x17, 230),
        input_ring: Color::rgba(0x60, 0xa5, 0xfa, 102),
        key: Color::rgb(0x1c, 0x19, 0x17),
        key_text: Color::rgb(0xd6, 0xd3, 0xd1),
        dot_edge: Color::rgba(0, 0, 0, 31),
        ring_gray: Color::rgb(0x9f, 0x9f, 0xa9),
        sidebar: Color::rgb(0x29, 0x25, 0x24),
        sidebar_rule: Color::rgb(0x44, 0x40, 0x3b),
        interactive: Color::rgba(0x1c, 0x19, 0x17, 166),
        interactive_hover: Color::rgba(0x1c, 0x19, 0x17, 89),
        scroll_thumb: Color::rgba(255, 255, 255, 51),
        // Dark paper is the hue pulled toward black: the middle of
        // `darkenHex(tint, 0.4)` and `darkenHex(tint, 0.55)`.
        paper: Shade::Darken(0.475),
    },
};
