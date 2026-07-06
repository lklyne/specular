// ADR 0008 — the single color picker for every canvas popup.
//
// The main popup shows only the active color as a trigger swatch + chevron
// (FigJam layout). Opening it drops a secondary popup below with the full
// palette row — one swatch per hue, a ring on the active one. base-ui's Menu
// gives us outside-click / Escape dismiss and positioning for free.

import { Menu } from '@base-ui/react/menu'
import { ChevronDown } from 'lucide-react'
import {
  paletteSlots,
  resolveCanvasColor,
  type CanvasColorRole,
  type CanvasColorSlot,
  type CanvasPalette,
} from '../../shared/canvas-colors'
import { swatchDotShadow, swatchRingShadow } from './colorSwatchStyle'

function triggerClass(isDark: boolean): string {
  const base =
    'flex h-6 items-center gap-1 rounded-[6px] border-0 pl-1 pr-1.5 transition-colors'
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100 data-[popup-open]:bg-[rgba(253,248,245,0.1)] data-[popup-open]:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900 data-[popup-open]:bg-[var(--color-stone-200)] data-[popup-open]:text-zinc-900`
}

export function ColorDropdown({
  isDark,
  palette,
  activeSlot,
  role,
  noun,
  onPick,
}: {
  isDark: boolean
  palette: CanvasPalette
  activeSlot: CanvasColorSlot | null
  role: CanvasColorRole
  /** Names the target for aria (e.g. "shape", "default brush"). */
  noun?: string
  onPick: (storage: string) => void
}) {
  const slots = paletteSlots(palette)
  const active = slots.find((slot) => slot.id === activeSlot)
  const activeColor = active
    ? active.hex ?? resolveCanvasColor(active.storage, { role, isDark })
    : null
  const triggerLabel = noun ? `Set ${noun} color` : 'Set color'

  return (
    <Menu.Root>
      <Menu.Trigger className={triggerClass(isDark)} aria-label={triggerLabel} title={triggerLabel}>
        <span
          className="block h-4 w-4 rounded-full"
          style={{
            background: activeColor ?? 'transparent',
            boxShadow: activeColor
              ? swatchDotShadow(isDark)
              : 'inset 0 0 0 1px var(--surface-popup-border)',
          }}
        />
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner align="center" side="bottom" sideOffset={8} style={{ zIndex: 50 }}>
          <Menu.Popup
            data-overlay-ui
            className="flex items-center gap-1 rounded-[10px] border p-1"
            style={{
              background: 'var(--surface-popup)',
              borderColor: 'var(--surface-popup-border)',
              boxShadow: isDark
                ? '0 10px 8px -6px rgba(0,0,0,.58), 0 4px 16px 0 rgba(0,0,0,.5)'
                : '0 10px 8px -6px rgba(0,0,0,.18), 0 4px 16px 0 rgba(199,193,188,.5)',
            }}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {slots.map((slot) => {
              const swatch = slot.hex ?? resolveCanvasColor(slot.storage, { role, isDark })
              const label = noun ? `Set ${noun} color to ${slot.label}` : `Set color to ${slot.label}`
              return (
                <Menu.Item
                  key={slot.id}
                  closeOnClick={false}
                  aria-label={label}
                  onClick={() => onPick(slot.storage)}
                  className="flex h-5 w-5 cursor-default items-center justify-center rounded-full outline-none transition-shadow bg-[var(--surface-popup)]"
                  style={{ boxShadow: swatchRingShadow(swatch, activeSlot === slot.id, isDark) }}
                >
                  <span
                    className="block h-3 w-3 rounded-full"
                    style={{ background: swatch, boxShadow: swatchDotShadow(isDark) }}
                  />
                </Menu.Item>
              )
            })}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
