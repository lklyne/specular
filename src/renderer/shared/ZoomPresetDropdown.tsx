import { type ReactElement, useMemo, useState } from 'react'
import { presetRowClass } from './PresetList'
import { PresetPopover, toolbarStripAnchor } from './PresetPopover'

// Zoom picker sharing the page-size dropdown's popover shell and row styling.
// Flat list of zoom levels; the 100% row surfaces its ⌘1 shortcut.
export function ZoomPresetDropdown({
  isDark,
  levels,
  activeLevel,
  onSelect,
  shortcutLevel,
  onOpenChange,
  trigger,
}: {
  isDark: boolean
  levels: readonly number[]
  activeLevel: number | null
  onSelect: (level: number) => void
  shortcutLevel?: number
  onOpenChange?: (open: boolean) => void
  trigger: ReactElement
}) {
  const [open, setOpen] = useState(false)
  const anchor = useMemo(() => toolbarStripAnchor('[data-zoom-anchor]'), [])

  function handle(next: boolean) {
    setOpen(next)
    onOpenChange?.(next)
  }

  function handleSelect(level: number) {
    onSelect(level)
    handle(false)
  }

  const kbdClass = isDark
    ? 'rounded-[4px] bg-[var(--color-stone-900)] px-1.5 py-0.5 text-xs leading-none text-[var(--color-stone-300)]'
    : 'rounded-[4px] bg-[var(--color-stone-200)] px-1.5 py-0.5 text-xs leading-none text-[var(--color-stone-600)]'

  // sideOffset 8 matches the add-page tool popup's gap below the toolbar.
  return (
    <PresetPopover isDark={isDark} open={open} onOpenChange={handle} anchor={anchor} sideOffset={8} trigger={trigger}>
      <div className="flex w-40 flex-col">
        {levels.map((level) => (
          <button
            key={level}
            type="button"
            className={presetRowClass(isDark, activeLevel === level)}
            aria-pressed={activeLevel === level}
            aria-label={`Zoom to ${level}%`}
            onClick={() => handleSelect(level)}
          >
            <span className="text-xs font-medium leading-none tabular-nums">{level}%</span>
            {level === shortcutLevel ? (
              <kbd className={kbdClass}>
                <span className="inline-flex items-center gap-1">
                  <span>⌘</span>
                  <span>1</span>
                </span>
              </kbd>
            ) : (
              <span />
            )}
          </button>
        ))}
      </div>
    </PresetPopover>
  )
}
