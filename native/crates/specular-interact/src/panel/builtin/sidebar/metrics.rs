//! The sizes of the sidebar in logical pixels, each from
//! `left-sidebar/App.tsx` and `SidebarCanvasTree.tsx`. A Tailwind unit is
//! 4 px.

use crate::SIDEBAR_WIDTH;

/// The right border of the aside: `border-r`.
pub(super) const EDGE: f32 = 1.0;
/// The width rows and heads are laid across: the aside less its border.
pub(super) const CONTENT: f32 = SIDEBAR_WIDTH - EDGE;
/// A section head: `h-9 px-3`.
pub(super) const HEAD: f32 = 36.0;
pub(super) const HEAD_PAD: f32 = 12.0;
/// Between a head's title and its chevron: `gap-1.5`.
pub(super) const HEAD_GAP: f32 = 6.0;
/// The head's title weight: `font-medium`.
pub(super) const HEAD_WEIGHT: u16 = 500;
/// The add button: `p-1.5` around a 14 px glyph, `rounded-[8px]`.
pub(super) const ADD: f32 = 26.0;
pub(super) const ADD_RADIUS: f32 = 8.0;
/// Between the Canvases head and the add button: `gap-1`.
pub(super) const ADD_GAP: f32 = 4.0;

/// A row: `py-1.5` around a 16 px line of `text-xs`.
pub(super) const ROW: f32 = 28.0;
/// A row's left padding, `LIST_OUTER_LEFT_PADDING + LIST_ROW_INNER_X_PADDING`,
/// and its right, `LIST_OUTER_RIGHT_PADDING + LIST_ROW_INNER_X_PADDING`.
pub(super) const PAD_LEFT: f32 = 22.0;
pub(super) const PAD_RIGHT: f32 = 16.0;
/// What each level of depth adds: `TREE_DEPTH_STEP`.
pub(super) const STEP: f32 = 14.0;
/// A row's glyph, and the gap after it: `size={14}`, `gap-2`.
pub(super) const ICON: f32 = 14.0;
pub(super) const GAP: f32 = 8.0;
/// A fold chevron: `size={12}`, 16 px left of the row's content.
pub(super) const FOLD: f32 = 12.0;
pub(super) const FOLD_LEFT: f32 = 16.0;

/// The room above the canvases `pt-0.5` and below them `pb-2`.
pub(super) const LIST_TOP: f32 = 2.0;
pub(super) const LIST_BOTTOM: f32 = 8.0;
/// Above and below the sections: `py-2`.
pub(super) const SECTIONS_PAD: f32 = 8.0;
/// The line over the sections: `border-t`.
pub(super) const RULE: f32 = 1.0;
/// The note on an empty canvas: `py-1` around a line.
pub(super) const EMPTY: f32 = 22.0;

/// The scrollbar: `::-webkit-scrollbar { width: 6px }`, its thumb rounded 3.
pub(super) const SCROLLBAR: f32 = 6.0;
pub(super) const SCROLLBAR_RADIUS: f32 = 3.0;
pub(super) const THUMB_MIN: f32 = 20.0;

/// The box around a name being edited: `-ml-0.5 px-0.5 py-0.5 ring-1`, so
/// it reaches 2 px left of the text, 2 above and below, and the ring takes
/// one more each side.
pub(super) const INPUT_LEFT: f32 = 3.0;
pub(super) const INPUT_TOP: f32 = 3.0;
pub(super) const INPUT_PAD: f32 = 2.0;
pub(super) const INPUT_HEIGHT: f32 = 22.0;
pub(super) const INPUT_RADIUS: f32 = 4.0;

/// The context menu: `min-w-40 p-1 border`, items `px-2.5 py-1.5` in
/// `rounded-[7px]`.
pub(super) const MENU_WIDTH: f32 = 160.0;
pub(super) const MENU_ITEM: f32 = 28.0;
pub(super) const MENU_INSET: f32 = 5.0;
pub(super) const MENU_PAD: f32 = 10.0;
pub(super) const MENU_RADIUS: f32 = 7.0;
