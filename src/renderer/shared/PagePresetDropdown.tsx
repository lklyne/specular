import { type ReactElement, useState } from 'react'
import { Popover } from '@base-ui/react/popover'
import { VIEWPORT_PRESETS } from '../../shared/constants'

export function PagePresetDropdown({
  align = 'center',
  isDark,
  onOpenChange,
  onSelectPreset,
  onSelectCustom,
  open: openProp,
  side = 'bottom',
  sideOffset = 4,
  trigger,
}: {
  align?: 'start' | 'center' | 'end'
  isDark: boolean
  onOpenChange?: (open: boolean) => void
  onSelectPreset: (index: number) => void
  onSelectCustom: () => void
  open?: boolean
  side?: 'top' | 'bottom' | 'left' | 'right'
  sideOffset?: number
  trigger: ReactElement
}) {
  const [localOpen, setLocalOpen] = useState(false)
  const isControlled = openProp !== undefined
  const open = isControlled ? openProp : localOpen

  function setOpen(next: boolean) {
    if (!isControlled) setLocalOpen(next)
    onOpenChange?.(next)
  }

  function handleSelect(callback: () => void) {
    callback()
    setOpen(false)
  }

  const popupClassName =
    'min-w-[240px] overflow-hidden rounded-md border border-[var(--surface-popup-border)] bg-[var(--surface-popup)] py-1 text-[var(--surface-toolbar-foreground)] shadow-xl'
  const itemClassName =
    'flex w-full cursor-pointer items-center justify-between gap-4 px-3 py-1.5 text-left text-xs text-[var(--surface-toolbar-foreground)] hover:bg-[var(--surface-interactive)]'

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger render={trigger} />
      <Popover.Portal>
        <Popover.Positioner side={side} align={align} sideOffset={sideOffset} collisionAvoidance={{ side: 'none', align: 'none' }} style={{ zIndex: 100 }}>
          <Popover.Popup data-overlay-ui className={popupClassName}>
            <button type="button" className={itemClassName} onClick={() => handleSelect(onSelectCustom)}>
              <span className="truncate">Custom</span>
            </button>
            {VIEWPORT_PRESETS.map((preset, index) => (
              <button
                key={preset.label}
                type="button"
                className={itemClassName}
                onClick={() => handleSelect(() => onSelectPreset(index))}
              >
                <span className="truncate">{preset.label}</span>
                <span className="shrink-0 text-[10px] tabular-nums text-zinc-500">
                  {preset.width}x{preset.height}
                </span>
              </button>
            ))}
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
