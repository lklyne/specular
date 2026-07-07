import { type ReactElement, useMemo, useState } from 'react'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import { presetRowClass } from './PresetList'
import { PresetPopover } from './PresetPopover'

// Anchors the popup to the toolbar strip's bottom edge (not the trigger button,
// which sits inset within the strip) so its gap below the toolbar matches the
// add-page tool popup — which anchors at the same y in the above-view overlay.
const stripBottomAnchor = {
  getBoundingClientRect: () => {
    const strip = document.querySelector('.toolbar-bar')?.getBoundingClientRect()
    const trigger = document.querySelector('[data-zoom-anchor]')?.getBoundingClientRect()
    const bottom = strip?.bottom ?? TOOLBAR_HEIGHT
    const left = trigger?.left ?? 0
    const width = trigger?.width ?? 0
    return new DOMRect(left, bottom, width, 0)
  },
}

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
  const anchor = useMemo(() => stripBottomAnchor, [])

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
