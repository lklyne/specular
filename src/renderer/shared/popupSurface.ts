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

// Shared trigger for the icon dropdowns (color / shape / border) that open a
// popup below. `padding` is passed as a literal class fragment (e.g. 'pl-1
// pr-1.5') so Tailwind can see it statically.
export function dropdownTriggerClass(isDark: boolean, padding: string): string {
  const base = `flex h-6 items-center gap-1 rounded-[6px] border-0 ${padding} transition-colors`
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100 data-[popup-open]:bg-[rgba(253,248,245,0.1)] data-[popup-open]:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900 data-[popup-open]:bg-[var(--color-stone-200)] data-[popup-open]:text-zinc-900`
}
