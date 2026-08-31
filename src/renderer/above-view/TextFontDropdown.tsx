// Labeled dropdown for per-entity typeface, sitting next to TextSizeDropdown.
//
// Three fixed options, no custom escape hatch — a font is a pick from what the
// app bundles, not a free value like a pixel size. Every label renders in the
// face it names, so the list previews itself.

import { Menu } from '@base-ui/react/menu'
import { Check, ChevronDown } from 'lucide-react'
import {
  TEXT_FONT_OPTIONS,
  TEXT_FONT_STACKS,
  type TextFont,
} from '../../shared/text-fonts'
import {
  POPUP_SURFACE_CLASS,
  dropdownTriggerClass,
  popupSurfaceStyle,
} from '../shared/popupSurface'

// The row height is fixed rather than content-driven, so the list stays uniform
// whatever the labels say.
function itemClass(isDark: boolean): string {
  const base =
    'flex h-7 cursor-default items-center justify-between gap-3 rounded-[7px] px-2 text-xs outline-none'
  return isDark
    ? `${base} text-[var(--surface-foreground)] data-[highlighted]:bg-zinc-800`
    : `${base} text-[var(--surface-foreground)] data-[highlighted]:bg-zinc-100`
}

export function TextFontDropdown({
  isDark,
  value,
  ariaLabel,
  onPick,
}: {
  isDark: boolean
  value: TextFont
  ariaLabel: string
  onPick: (font: TextFont) => void
}) {
  const active = TEXT_FONT_OPTIONS.find((option) => option.value === value)
  return (
    <Menu.Root>
      <Menu.Trigger
        className={`${dropdownTriggerClass(isDark, 'px-1.5')} text-xs leading-none`}
        aria-label={ariaLabel}
        title={ariaLabel}
      >
        <span
          className="inline-block leading-none"
          style={{ fontFamily: TEXT_FONT_STACKS[value] }}
        >
          {active?.label}
        </span>
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner align="start" sideOffset={6} style={{ zIndex: 50 }}>
          <Menu.Popup
            data-overlay-ui
            className={`min-w-[120px] outline-none ${POPUP_SURFACE_CLASS}`}
            style={popupSurfaceStyle(isDark)}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {TEXT_FONT_OPTIONS.map((option) => (
              <Menu.Item
                key={option.value}
                className={itemClass(isDark)}
                onClick={() => onPick(option.value)}
              >
                <span
                  className="inline-block leading-none"
                  style={{ fontFamily: TEXT_FONT_STACKS[option.value] }}
                >
                  {option.label}
                </span>
                <span className="flex w-3 items-center justify-center">
                  {option.value === value ? <Check size={12} /> : null}
                </span>
              </Menu.Item>
            ))}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
