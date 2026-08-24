// --- Layout dedup cache ---
// Tracks the last-rendered state keys to avoid redundant view updates.
// Consumers read and write fields directly on the exported object.

export const layoutCache = {
  toolbarHeight: 44,
  layoutTimer: null as NodeJS.Immediate | null,
  // Bounds keys — cheap string comparisons for Electron view positioning
  lastBackgroundBoundsKey: null as string | null,
  lastLeftSidebarBoundsKey: null as string | null,
  lastToolbarBoundsKey: null as string | null,
  lastOverlayBoundsKey: null as string | null,
  lastAboveViewBoundsKey: null as string | null,
  lastCommentOverlayBoundsKey: null as string | null,
  lastCursorOverlayBoundsKey: null as string | null,
  lastFloatingUiBoundsKey: null as string | null,
  lastDevtoolsBackgroundBoundsKey: null as string | null,
  lastDevtoolsHeaderBoundsKey: null as string | null,
  lastDevtoolsResizeBoundsKey: null as string | null,
}

/** Reset all dedup keys to null (used during workspace transitions). */
export function resetLayoutCache(): void {
  layoutCache.lastBackgroundBoundsKey = null
  layoutCache.lastLeftSidebarBoundsKey = null
  layoutCache.lastToolbarBoundsKey = null
  layoutCache.lastOverlayBoundsKey = null
  layoutCache.lastAboveViewBoundsKey = null
  layoutCache.lastCommentOverlayBoundsKey = null
  layoutCache.lastCursorOverlayBoundsKey = null
  layoutCache.lastFloatingUiBoundsKey = null
  layoutCache.lastDevtoolsBackgroundBoundsKey = null
  layoutCache.lastDevtoolsHeaderBoundsKey = null
  layoutCache.lastDevtoolsResizeBoundsKey = null
}
