// Labeled dropdown for the `brush` draw type's webgpu-brush preset, sitting
// next to the color picker. `value` is null when a multi-selection mixes presets.

import { Menu } from '@base-ui/react/menu'
import { Check, ChevronDown } from 'lucide-react'
import {
  NATURAL_BRUSH_PRESETS,
  NATURAL_BRUSH_PRESET_LABELS,
  type NaturalBrushPreset,
} from '../../shared/natural-brush'
import {
  POPUP_SURFACE_CLASS,
  dropdownTriggerClass,
  popupSurfaceStyle,
} from '../shared/popupSurface'

function itemClass(isDark: boolean): string {
  const base =
    'flex h-7 cursor-default items-center justify-between gap-3 rounded-[7px] px-2 text-xs outline-none'
  return isDark
    ? `${base} text-[var(--surface-foreground)] data-[highlighted]:bg-zinc-800`
    : `${base} text-[var(--surface-foreground)] data-[highlighted]:bg-zinc-100`
}

export function BrushPresetDropdown({
  isDark,
  value,
  ariaLabel,
  onPick,
}: {
  isDark: boolean
  value: NaturalBrushPreset | null
  ariaLabel: string
  onPick: (preset: NaturalBrushPreset) => void
}) {
  return (
    <Menu.Root>
      <Menu.Trigger
        className={`${dropdownTriggerClass(isDark, 'px-1.5')} text-xs leading-none`}
        aria-label={ariaLabel}
        title={ariaLabel}
      >
        <span className="inline-block leading-none">
          {value ? NATURAL_BRUSH_PRESET_LABELS[value] : 'Mixed'}
        </span>
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner align="start" sideOffset={6} style={{ zIndex: 50 }}>
          <Menu.Popup
            data-overlay-ui
            className={`min-w-[140px] outline-none ${POPUP_SURFACE_CLASS}`}
            style={popupSurfaceStyle(isDark)}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {NATURAL_BRUSH_PRESETS.map((preset) => (
              <Menu.Item
                key={preset}
                className={itemClass(isDark)}
                onClick={() => onPick(preset)}
              >
                <span className="inline-block leading-none">
                  {NATURAL_BRUSH_PRESET_LABELS[preset]}
                </span>
                <span className="flex w-3 items-center justify-center">
                  {preset === value ? <Check size={12} /> : null}
                </span>
              </Menu.Item>
            ))}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
