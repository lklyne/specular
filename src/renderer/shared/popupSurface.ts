import type { CSSProperties } from 'react'

// Source of truth for the floating popup box shared by the add-page tool popup
// (CanvasItemPopup.Frame) and the toolbar/page preset dropdowns (PresetPopover).
// Kept as class + inline style so both renderers render an identical surface.
export const POPUP_SURFACE_CLASS = 'rounded-[10px] border p-1'

export function popupSurfaceStyle(isDark: boolean): CSSProperties {
  return {
    background: 'var(--surface-popup)',
    borderColor: 'var(--surface-popup-border)',
    boxShadow: isDark
      ? '0 10px 8px -6px rgba(0,0,0,.58), 0 4px 16px 0 rgba(0,0,0,.5)'
      : '0 10px 8px -6px rgba(0,0,0,.18), 0 4px 16px 0 rgba(199,193,188,.5)',
  }
}
