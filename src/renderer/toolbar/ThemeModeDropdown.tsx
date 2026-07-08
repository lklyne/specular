import { type ReactElement, useMemo, useState } from 'react'
import { Check } from 'lucide-react'
import type { AppThemeMode } from '../../shared/types'
import { presetRowClass } from '../shared/PresetList'
import { PresetPopover, toolbarStripAnchor } from '../shared/PresetPopover'

const THEME_MODES: { mode: AppThemeMode; label: string }[] = [
  { mode: 'system', label: 'System' },
  { mode: 'light', label: 'Light' },
  { mode: 'dark', label: 'Dark' },
]

// Theme picker sharing the zoom/page-size dropdowns' popover shell and row
// styling (PresetPopover + presetRowClass). Flat list of the three theme
// modes; the active one carries a checkmark.
export function ThemeModeDropdown({
  isDark,
  activeMode,
  onSelect,
  onOpenChange,
  trigger,
}: {
  isDark: boolean
  activeMode: AppThemeMode
  onSelect: (mode: AppThemeMode) => void
  onOpenChange?: (open: boolean) => void
  trigger: ReactElement
}) {
  const [open, setOpen] = useState(false)
  const anchor = useMemo(() => toolbarStripAnchor('[data-theme-anchor]'), [])

  function handle(next: boolean) {
    setOpen(next)
    onOpenChange?.(next)
  }

  function handleSelect(mode: AppThemeMode) {
    onSelect(mode)
    handle(false)
  }

  // sideOffset 8 matches the add-page tool popup's gap below the toolbar.
  return (
    <PresetPopover isDark={isDark} open={open} onOpenChange={handle} anchor={anchor} sideOffset={8} trigger={trigger}>
      <div className="flex w-32 flex-col">
        {THEME_MODES.map(({ mode, label }) => (
          <button
            key={mode}
            type="button"
            className={presetRowClass(isDark, activeMode === mode)}
            aria-pressed={activeMode === mode}
            aria-label={`Use ${label.toLowerCase()} theme`}
            onClick={() => handleSelect(mode)}
          >
            <span className="text-xs font-medium leading-none">{label}</span>
            {activeMode === mode ? <Check size={12} /> : <span />}
          </button>
        ))}
      </div>
    </PresetPopover>
  )
}
