// Edge routing picker — mirrors ShapeDropdown. Used by the edge selection
// popup and by the connect tool's popup (where it writes a tool default).

import { Menu } from '@base-ui/react/menu'
import { ChevronDown } from 'lucide-react'
import type { EdgeRouting } from '../../shared/types'
import { POPUP_SURFACE_CLASS, dropdownTriggerClass, popupSurfaceStyle } from '../shared/popupSurface'

const ROUTING_OPTIONS: { value: EdgeRouting; label: string; d: string }[] = [
  { value: 'elbow', label: 'Elbow', d: 'M2 4 H8 V12 H14' },
  { value: 'straight', label: 'Straight', d: 'M2 4 L14 12' },
  { value: 'bezier', label: 'Curved', d: 'M2 4 C8 4, 8 12, 14 12' },
]

function RoutingGlyph({ d, size = 16 }: { d: string; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden>
      <path d={d} stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
    </svg>
  )
}

function cellClass(isDark: boolean, active: boolean): string {
  const base =
    'flex h-8 w-8 cursor-default items-center justify-center rounded-md outline-none transition-colors'
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.14)] text-[var(--surface-foreground)]`
      : `${base} bg-[var(--color-stone-200)] text-[var(--surface-foreground)]`
  }
  return isDark
    ? `${base} text-[var(--surface-foreground-muted)] hover:bg-[rgba(253,248,245,0.1)] hover:text-[var(--surface-foreground)]`
    : `${base} text-[var(--surface-foreground-muted)] hover:bg-[var(--color-stone-100)] hover:text-[var(--surface-foreground)]`
}

export function RoutingDropdown({
  isDark,
  routing,
  noun,
  onPick,
}: {
  isDark: boolean
  routing: EdgeRouting
  /** Names the target for aria (e.g. "edge", "default"). */
  noun?: string
  onPick: (routing: EdgeRouting) => void
}) {
  const active = ROUTING_OPTIONS.find((option) => option.value === routing) ?? ROUTING_OPTIONS[0]
  const triggerLabel = noun ? `Set ${noun} routing` : 'Set routing'
  return (
    <Menu.Root>
      <Menu.Trigger
        className={dropdownTriggerClass(isDark, 'px-1.5')}
        aria-label={triggerLabel}
        title={triggerLabel}
      >
        <RoutingGlyph d={active.d} />
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner align="center" side="bottom" sideOffset={8} style={{ zIndex: 50 }}>
          <Menu.Popup
            data-overlay-ui
            className={`flex gap-0.5 ${POPUP_SURFACE_CLASS}`}
            style={popupSurfaceStyle(isDark)}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {ROUTING_OPTIONS.map((option) => (
              <Menu.Item
                key={option.value}
                closeOnClick
                aria-label={noun ? `Set ${noun} routing to ${option.label}` : option.label}
                title={option.label}
                className={cellClass(isDark, option.value === routing)}
                onClick={() => onPick(option.value)}
              >
                <RoutingGlyph d={option.d} />
              </Menu.Item>
            ))}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
