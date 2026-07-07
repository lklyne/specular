import { type ReactElement, useState } from 'react'
import { PresetList } from './PresetList'
import { PresetPopover } from './PresetPopover'

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
  hideCustom = false,
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
  hideCustom?: boolean
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

  return (
    <PresetPopover isDark={isDark} open={open} onOpenChange={setOpen} side={side} align={align} sideOffset={sideOffset} trigger={trigger}>
      <PresetList
        isDark={isDark}
        activePreset={activePreset}
        customActive={customActive}
        onSelectPreset={(index) => handleSelect(() => onSelectPreset(index))}
        onSelectCustom={() => handleSelect(onSelectCustom)}
        hideCustom={hideCustom}
      />
    </PresetPopover>
  )
}
