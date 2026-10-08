//! The sizes of the built-in panels, in logical pixels, each from the
//! Electron app's CSS. A Tailwind unit is 4 px.

/// The toolbar strip: `h-[44px]` in `toolbar/App.tsx`, and `TOOLBAR_HEIGHT`
/// in `shared/constants.ts`.
pub const TOOLBAR_HEIGHT: f32 = 44.0;
/// A tool button: `h-7 w-8` in `toolbarToolBtnClass`.
pub(super) const TOOL_BUTTON: (f32, f32) = (32.0, 28.0);
/// A tool glyph: `TOOL_GLYPH_SIZE`.
pub(super) const TOOL_GLYPH: f32 = 20.0;
/// The zoom readout: `h-7 w-[58px] border pl-2 pr-1`, with a 10 px
/// chevron. The border is transparent but takes a pixel of room.
pub(super) const ZOOM_TRIGGER: (f32, f32) = (58.0, 28.0);
pub(super) const ZOOM_PAD: (f32, f32) = (9.0, 5.0);
pub(super) const ZOOM_CHEVRON: f32 = 10.0;
/// The corner of a tool button and of a popup control: `rounded-[6px]`.
pub(super) const CONTROL_RADIUS: f32 = 6.0;

/// The space between neighbours in a row: `gap-1`.
pub(super) const GAP: f32 = 4.0;
/// A divider between groups: `mx-1 h-4 w-px`.
pub(super) const DIVIDER: (f32, f32) = (1.0, 16.0);
pub(super) const DIVIDER_MARGIN: f32 = 4.0;
/// A divider inside a dropdown's row: `mx-0.5 h-5 w-px`.
pub(super) const RULE: (f32, f32) = (1.0, 20.0);
pub(super) const RULE_MARGIN: f32 = 2.0;

/// From a floating panel's edge to its content: `border p-1` in
/// `POPUP_SURFACE_CLASS`.
pub(super) const INSET: f32 = 5.0;
/// How near a viewport edge a panel may come: `POPUP_EDGE_MARGIN`.
pub(super) const EDGE_MARGIN: f32 = 8.0;

/// The height of a text field: `border px-2 py-1 text-xs`, a 16 px line
/// between a pixel of border and 4 px of padding each side. Its side padding
/// is `FIELD_PAD`. The address field is at least `URL_INPUT_MIN_WIDTH` wide;
/// a number is a few characters.
pub const FIELD_HEIGHT: f32 = 26.0;
pub(super) const FIELD_PAD: f32 = 8.0;
pub(super) const FIELD_WIDE: f32 = 280.0;
pub(super) const FIELD_SHORT: f32 = 56.0;
/// The size of a field's text.
pub const FIELD_TEXT: f32 = 12.0;
/// The height of a field's line of text.
pub const FIELD_LINE: f32 = 16.0;

/// A popup control: `h-6 w-6` in `popupIconButtonClass`.
pub(super) const CONTROL: f32 = 24.0;
/// A popup glyph: `size={14}` on nearly every icon.
pub(super) const ICON: f32 = 14.0;
/// The chevron of a dropdown: `<ChevronDown size={12} />`.
pub(super) const CHEVRON: f32 = 12.0;
/// The chevron of a page size: `<ChevronDown size={10} />`.
pub(super) const PRESET_CHEVRON: f32 = 10.0;
/// A swatch and its dot: `h-5 w-5` around `h-3 w-3` in `ColorSwatch`.
pub(super) const SWATCH: f32 = 20.0;
pub(super) const DOT: f32 = 12.0;
/// The dot on a closed color dropdown: `h-4 w-4` in `ColorDropdown`.
pub(super) const TRIGGER_DOT: f32 = 16.0;
/// Panel text: `text-xs`, 12 px on a 16 px line.
pub(super) const TEXT_SIZE: f32 = 12.0;
pub(super) const TEXT_LINE: f32 = 16.0;
/// The padding of a labelled toggle: `px-2` in `segmentClass`.
pub(super) const SEGMENT_PAD: f32 = 8.0;
/// The least width of a stepper's number.
pub(super) const STEPPER_VALUE: f32 = 44.0;

/// A list row: `px-2 py-1 text-xs` in `TextSizeDropdown`, `h-7` when it
/// previews a typeface (`TextFontDropdown`), `rounded-[7px]`.
pub(super) const ROW: f32 = 24.0;
pub(super) const ROW_TALL: f32 = 28.0;
pub(super) const ROW_PAD: f32 = 8.0;
pub(super) const ROW_RADIUS: f32 = 7.0;
/// Between a row's label and what is at its far end: `gap-3`.
pub(super) const ROW_GAP: f32 = 12.0;
/// The least width of a list: `min-w-[140px]`, or `min-w-[120px]` for
/// typefaces.
pub(super) const LIST_MIN: f32 = 140.0;
pub(super) const FONT_LIST_MIN: f32 = 120.0;
/// A list of presets: `w-40` for zoom levels (`ZoomPresetDropdown`), `w-56`
/// when each row ends in a size (`PresetList`).
pub(super) const PRESETS: f32 = 160.0;
pub(super) const PRESETS_WIDE: f32 = 224.0;
/// The key hint at the end of a preset row: `px-1.5 py-0.5`.
pub(super) const KEY_PAD: (f32, f32) = (6.0, 2.0);
/// A line between a list's sections, with its margins: `my-1 h-px`.
pub(super) const SECTION_GAP: f32 = 9.0;
/// The same between two rows of controls: `gap-1.5` around `my-0.5 h-px`.
pub(super) const CONTROLS_GAP: f32 = 17.0;
/// A line that stops short of the panel's edges: `mx-1`.
pub(super) const SECTION_RULE_INSET: f32 = 4.0;
/// Around a stepper under a list: `px-1 pb-1`.
pub(super) const STEPPER_INSET: f32 = 4.0;

/// A list of segments, its swatches under them: `w-[300px]` less the frame's
/// border and padding.
pub(super) const SEGMENTED_ROW: f32 = 300.0 - INSET * 2.0;

/// A cell of a row of glyph options: `h-7 w-7` around `size={15}` in
/// `TextAlignDropdown`.
pub(super) const CELL: f32 = 28.0;
pub(super) const CELL_ICON: f32 = 15.0;
/// A cell of a grid of shapes: `h-8 w-8` around an 18 px glyph, `gap-0.5`
/// (`ShapeDropdown`).
pub(super) const GRID_CELL: f32 = 32.0;
pub(super) const GRID_ICON: f32 = 18.0;
pub(super) const GRID_GAP: f32 = 2.0;
/// The shape on a closed shape dropdown: `<ShapeGlyph size={16} />`.
pub(super) const TRIGGER_SHAPE: f32 = 16.0;

/// From a trigger to its list: `sideOffset={8}`, or `{6}` for a list of
/// words.
pub(super) const LIST_OFFSET: f32 = 8.0;
pub(super) const MENU_OFFSET: f32 = 6.0;
/// The page size list keeps `PagePresetDropdown`'s default of 4.
pub(super) const PRESET_OFFSET: f32 = 4.0;
