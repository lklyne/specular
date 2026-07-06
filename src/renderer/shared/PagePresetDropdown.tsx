import { type ReactElement, useState } from 'react'
import { Popover } from '@base-ui/react/popover'
import { PresetList } from './PresetList'

export function PagePresetDropdown({
  align = 'center',
  isDark,
  activePreset = null,
  customActive = false,
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
  activePreset?: number | null
  customActive?: boolean
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
    'overflow-hidden rounded-md border border-[var(--surface-popup-border)] bg-[var(--surface-popup)] p-1 text-[var(--surface-toolbar-foreground)] shadow-xl'

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger render={trigger} />
      <Popover.Portal>
        <Popover.Positioner side={side} align={align} sideOffset={sideOffset} collisionAvoidance={{ side: 'none', align: 'none' }} style={{ zIndex: 100 }}>
          <Popover.Popup data-overlay-ui className={popupClassName}>
            <PresetList
              isDark={isDark}
              activePreset={activePreset}
              customActive={customActive}
              onSelectPreset={(index) => handleSelect(() => onSelectPreset(index))}
              onSelectCustom={() => handleSelect(onSelectCustom)}
            />
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
