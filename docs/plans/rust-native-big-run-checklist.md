# Rust native big run: by-hand checklist

**Status:** Filled in as the run lands. Walk it once at the end of the [big run](./rust-native-big-run.md).

## The context menu (P4a)

The Kit draws it now; before, the canvas pass did.

- Right-click a sticky: a menu opens with its corner at the pointer, in the same style as the sidebar's canvas-row menu, in light and in dark. The sticky becomes the selection. Sections are split by lines; Cut, Copy, Paste and Duplicate show their keys (⌘X, ⌘C, ⌘V, ⌘D) muted at the right.
- Right-click a page: Back, Forward and Reload come first; Back is greyed on a page with no history and cannot be clicked.
- Right-click empty canvas: Paste and Select all. Paste puts the copied item where the menu opened, not where the Paste row is.
- Right-click near the bottom-right corner: the menu stays inside the window.
- Arrow keys move through the rows, Return picks, Escape closes. Nothing reaches the canvas while it is open: the arrows do not nudge the selection.
- Click outside the menu (on the canvas, the toolbar, the sidebar): it closes and nothing else happens. No marquee starts and the selection stays.
- Scroll or pinch while it is open: the canvas does not move.
- Right-click a canvas row in the sidebar: its own Rename and Delete menu, as before.
