// ADR 0008 — the single color picker for every canvas popup.
//
// The main popup shows only the active color as a trigger swatch + chevron
// (FigJam layout). Opening it drops a secondary popup below with the full
// palette row — one swatch per hue, a ring on the active one. base-ui's Menu
// gives us outside-click / Escape dismiss and positioning for free.

import { Menu } from '@base-ui/react/menu'
import { Ban, ChevronDown } from 'lucide-react'
import {
  paletteSlots,
  resolveCanvasColor,
  type CanvasColorRole,
  type CanvasColorSlot,
  type CanvasPalette,
} from '../../shared/canvas-colors'
import { swatchDotShadow, swatchRingShadow } from './colorSwatchStyle'
import {
  POPUP_SURFACE_CLASS,
  dropdownTriggerClass,
  popupSurfaceStyle,
} from '../shared/popupSurface'

export function ColorDropdown({
  isDark,
  palette,
  activeSlot,
  role,
  noun,
  transparentActive = false,
  onPickTransparent,
  onPick,
}: {
  isDark: boolean
  palette: CanvasPalette
  activeSlot: CanvasColorSlot | null
  role: CanvasColorRole
  /** Names the target for aria (e.g. "shape", "default brush"). */
  noun?: string
  /** Adds a circle-slash swatch for controls that support no paint. */
  transparentActive?: boolean
  onPickTransparent?: () => void
  onPick: (storage: string) => void
}) {
  const slots = paletteSlots(palette)
  const active = slots.find((slot) => slot.id === activeSlot)
  const activeColor = active
    ? (active.hex ?? resolveCanvasColor(active.storage, { role, isDark }))
    : null
  const triggerLabel = noun ? `Set ${noun} color` : 'Set color'

  return (
    <Menu.Root>
      <Menu.Trigger
        className={dropdownTriggerClass(isDark, 'pl-1 pr-1.5')}
        aria-label={triggerLabel}
        title={triggerLabel}
      >
        {transparentActive ? (
          <span className="flex h-4 w-4 items-center justify-center rounded-full bg-[var(--surface-popup)] shadow-[inset_0_0_0_1px_var(--surface-popup-border)]">
            <Ban size={12} />
          </span>
        ) : (
          <span
            className="block h-4 w-4 rounded-full"
            style={{
              background: activeColor ?? 'transparent',
              boxShadow: activeColor
                ? swatchDotShadow(isDark)
                : 'inset 0 0 0 1px var(--surface-popup-border)',
            }}
          />
        )}
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner
          align="center"
          side="bottom"
          sideOffset={8}
          style={{ zIndex: 50 }}
        >
          <Menu.Popup
            data-overlay-ui
            className={`flex items-center gap-1 ${POPUP_SURFACE_CLASS}`}
            style={popupSurfaceStyle(isDark)}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {onPickTransparent ? (
              <Menu.Item
                closeOnClick={false}
                aria-label={
                  noun
                    ? `Set ${noun} color to transparent`
                    : 'Set color to transparent'
                }
                onClick={onPickTransparent}
                className="flex h-5 w-5 cursor-default items-center justify-center rounded-full bg-[var(--surface-popup)] outline-none transition-shadow"
                style={{
                  boxShadow: swatchRingShadow(
                    isDark ? '#a1a1aa' : '#71717a',
                    transparentActive,
                    isDark,
                  ),
                }}
              >
                <span className="flex h-3 w-3 items-center justify-center rounded-full">
                  <Ban size={12} />
                </span>
              </Menu.Item>
            ) : null}
            {slots.map((slot) => {
              const swatch =
                slot.hex ?? resolveCanvasColor(slot.storage, { role, isDark })
              const label = noun
                ? `Set ${noun} color to ${slot.label}`
                : `Set color to ${slot.label}`
              return (
                <Menu.Item
                  key={slot.id}
                  closeOnClick={false}
                  aria-label={label}
                  onClick={() => onPick(slot.storage)}
                  className="flex h-5 w-5 cursor-default items-center justify-center rounded-full outline-none transition-shadow bg-[var(--surface-popup)]"
                  style={{
                    boxShadow: swatchRingShadow(
                      swatch,
                      !transparentActive && activeSlot === slot.id,
                      isDark,
                    ),
                  }}
                >
                  <span
                    className="block h-3 w-3 rounded-full"
                    style={{
                      background: swatch,
                      boxShadow: swatchDotShadow(isDark),
                    }}
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
