//! [`Colors`]: every colour `view` and the built-in panels draw with, one
//! set per [`Appearance`].
//!
//! The light values are Electron's light tokens
//! (`src/renderer/shared/surfaceTheme.css`, the Tailwind classes of the
//! toolbar and popups, `canvas-colors.ts`). The dark ones are its `.dark`
//! block and the `isDark` branches of the canvas renderers, with Tailwind's
//! stone and zinc scales resolved to sRGB. Nothing drawn picks a colour
//! anywhere else.

mod dark;
mod light;

use specular_interact::Appearance;

use crate::{Blend, Color};

/// How a fill is derived from its hue: toward white on the light canvas,
/// toward black on the dark one (`ShapeBodyLayer`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shade {
    /// Moved this far toward white.
    Lighten(f32),
    /// Moved this far toward black.
    Darken(f32),
    /// Left as it is.
    Keep,
}

/// The seven hues of the vivid palette, which paints glyphs and strokes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hues {
    /// Preset 1.
    pub red: Color,
    /// Preset 2.
    pub orange: Color,
    /// Preset 3.
    pub yellow: Color,
    /// Preset 4.
    pub green: Color,
    /// Preset 5.
    pub cyan: Color,
    /// Preset 6.
    pub purple: Color,
    /// The seventh, this app's own.
    pub blue: Color,
}

/// What the canvas and the things on it are drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    /// Which theme this is.
    pub appearance: Appearance,
    /// The canvas under the dots.
    pub canvas: Color,
    /// The dots.
    pub dot: Color,
    /// How faint a dense grid's dots may get: the floor of their opacity.
    pub dot_floor: f32,
    /// Default text, an edge's label, a Document's text.
    pub ink: Color,
    /// Text on a sticky note, whose paper is pale in both themes.
    pub paper_ink: Color,
    /// The neutral slot as a fill: an uncoloured sticky or shape.
    pub neutral_fill: Color,
    /// The vivid hues.
    pub hues: Hues,
    /// Selection outlines, handles, the marquee and guides.
    pub selection: Color,
    /// The highlight behind selected text.
    pub text_selection: Color,
    /// A page's resting border.
    pub page_border: Color,
    /// The bezel of a page's device frame.
    pub device_bezel: Color,
    /// The shadow a device frame drops.
    pub device_shadow: Color,
    /// A phone's notch.
    pub device_notch: Color,
    /// The home indicator in a phone's or a tablet's bezel.
    pub device_indicator: Color,
    /// The hairline in the bezel around the screen.
    pub device_screen_ring: Color,
    /// Page titles and file names.
    pub muted_text: Color,
    /// The inside of a file card and a Document.
    pub card: Color,
    /// The alpha of the shadow under a sticky note and a card.
    pub card_shadow: u8,
    /// The glyph on a file card that has no preview.
    pub file_glyph: Color,
    /// A link in a Document.
    pub link: Color,
    /// An untinted group's fill.
    pub group_fill: Color,
    /// An untinted group's border.
    pub group_border: Color,
    /// An untinted group's title.
    pub group_title: Color,
    /// A tinted group's title.
    pub group_tinted_title: Color,
    /// How much of the hue a tinted group's fill is.
    pub group_tint: f32,
    /// What a tinted group's border is mixed toward, and how much of the
    /// hue stays.
    pub group_border_mix: (Color, f32),
    /// A shape's fill from its hue.
    pub shape_fill: Shade,
    /// A shape's border from its hue.
    pub shape_border: Shade,
    /// A shape's label.
    pub shape_label: Color,
    /// How the highlighter lays its ink on what it marks. Multiplied, it
    /// tints pale paper and leaves dark text as dark as it was; on a dark
    /// canvas that would be black, so it is painted over instead.
    pub highlight: Blend,
    /// The comment composer's fill.
    pub composer: Color,
    /// Its border.
    pub composer_border: Color,
    /// Its text.
    pub composer_ink: Color,
    /// Its hint.
    pub composer_hint: Color,
    /// The built-in toolbar, popups and sidebar.
    pub panel: PanelColors,
}

/// The chrome (`--surface-*`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelColors {
    /// The toolbar strip.
    pub toolbar: Color,
    /// The line under the toolbar.
    pub toolbar_border: Color,
    /// A tool button that is hovered or on.
    pub tool_fill: Color,
    /// Toolbar text at rest and when hovered.
    pub toolbar_text: Color,
    /// Toolbar text when hovered or on.
    pub toolbar_text_strong: Color,
    /// The chevron beside the zoom readout.
    pub toolbar_chevron: Color,
    /// A floating panel.
    pub popup: Color,
    /// Its border.
    pub popup_border: Color,
    /// The tight shadow under it.
    pub shadow_near: Color,
    /// The wide one.
    pub shadow_far: Color,
    /// A menu of words.
    pub menu: Color,
    /// Its border.
    pub menu_border: Color,
    /// Its shadow.
    pub menu_shadow: Color,
    /// Panel text.
    pub text: Color,
    /// Quiet panel text.
    pub text_muted: Color,
    /// A hovered control.
    pub hover: Color,
    /// A control that is on.
    pub on: Color,
    /// The highlighted row of a menu.
    pub menu_hover: Color,
    /// A divider between groups.
    pub divider: Color,
    /// A line between sections of a list.
    pub rule: Color,
    /// The outline of a field.
    pub field_border: Color,
    /// A text field's fill.
    pub input: Color,
    /// The ring of a text field that has the keys.
    pub input_ring: Color,
    /// A key hint's fill.
    pub key: Color,
    /// A key hint's text.
    pub key_text: Color,
    /// The hairline around a swatch's dot.
    pub dot_edge: Color,
    /// The ring of a selected swatch too pale to ring itself.
    pub ring_gray: Color,
    /// The sidebar's ground.
    pub sidebar: Color,
    /// A line inside the sidebar.
    pub sidebar_rule: Color,
    /// A selected row, and a pressed button.
    pub interactive: Color,
    /// A hovered row or button.
    pub interactive_hover: Color,
    /// The thumb of a scrollbar.
    pub scroll_thumb: Color,
    /// How far the paper of the sticky and shape glyphs is taken from the
    /// tool's colour.
    pub paper: Shade,
}

impl Colors {
    /// The colours of `appearance`.
    #[must_use]
    pub const fn of(appearance: Appearance) -> &'static Self {
        match appearance {
            Appearance::Light => &light::COLORS,
            Appearance::Dark => &dark::COLORS,
        }
    }
}
