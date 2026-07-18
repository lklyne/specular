# PR #349 — Canvas selection, stack-order menus, popup gestures

Branch: `cursor/selection-z-order-gestures-b62b`
https://github.com/lklyne/specular/pull/349

## 1. Marquee selection modes

- Drag a marquee that only partially overlaps two items → both should select (intersect mode, default).
- Hold Cmd (or Ctrl) and drag the same marquee → only items **fully enclosed** should select (contain mode).
- bug: i still see the blue selection outline when holding command, even though an item wont get selected. another bug: i cant press command during a drag

## 2. Cmd-drag through an item body all works!

- Cmd-drag starting with the pointer *on top of* an unselected item (not empty canvas) → should start a marquee, not move/select that item.
- Release without moving (stationary click) while Cmd held on that item → should still fall back to a normal click/select on that item (the `originEntity` fallback).
- Confirm the origin item itself is excluded from the marquee's own selection outline while dragging over it.

## 3. Stack-order context menu

- Right-click a canvas item (shape, page, etc.) → verify menu shows **Bring forward / Send backward / Bring to front / Send to back** with correct keyboard accelerators shown.
- Trigger each item from the menu and confirm z-order updates correctly.
- Trigger the same actions via keyboard shortcuts (Cmd+], Cmd+[, Cmd+Shift+], Cmd+Shift+[) and confirm they match the menu behavior (this PR sources both from the same binding registry, so verify they didn't drift).
- Right-click an item, then delete that item via another path before the menu handler fires (edge case) — shouldn't crash (there's a new `currentEntityIds().has(entityId)` guard).

## 4. Wheel/pan through popups

- Open a canvas-item popup/toolbar (e.g. text formatting popup) so it's visible on screen.
- Scroll the wheel while cursor is over the popup → canvas should zoom/pan underneath instead of being blocked.
- Middle-click-drag starting on the popup → canvas should pan (verify it doesn't accidentally trigger popup buttons — left-click still should stay captured by the popup).
- Left-click a button inside the popup → should still work normally (not pass through to canvas).

## 5. Regression check

- Normal (no-modifier) drag-select and click-select on canvas items still behaves as before.
- Edge selection (click on an edge) still works — `data-edge-id` was added to EdgeLayer, worth confirming edge right-click/selection is unaffected.
