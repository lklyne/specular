// Shape picker for the canvas popups — mirrors ColorDropdown. The trigger
// shows the active shape's glyph + chevron; opening drops a grid of every
// shape below (base-ui Menu gives outside-click / Escape / positioning).

import { Menu } from '@base-ui/react/menu'
import { ChevronDown } from 'lucide-react'
import { SHAPE_DEFS, shapeDef } from '../../shared/shapes'
import { ShapeGlyph } from '../shared/ShapeGlyph'

function triggerClass(isDark: boolean): string {
  const base =
    'flex h-6 items-center gap-1 rounded-[6px] border-0 pl-1.5 pr-1.5 transition-colors'
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100 data-[popup-open]:bg-[rgba(253,248,245,0.1)] data-[popup-open]:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900 data-[popup-open]:bg-[var(--color-stone-200)] data-[popup-open]:text-zinc-900`
}

function cellClass(isDark: boolean, active: boolean): string {
  const base =
    'flex h-8 w-8 cursor-default items-center justify-center rounded-md outline-none transition-colors'
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.14)] text-zinc-100`
      : `${base} bg-[var(--color-stone-200)] text-zinc-900`
  }
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900`
}

export function ShapeDropdown({
  isDark,
  activeKind,
  noun,
  onPick,
}: {
  isDark: boolean
  activeKind: string | null
  /** Names the target for aria (e.g. "shape", "3 shapes", "default shape"). */
  noun?: string
  onPick: (kind: string) => void
}) {
  const active = activeKind ? shapeDef(activeKind) : null
  const triggerLabel = noun ? `Set ${noun} shape` : 'Set shape'
  return (
    <Menu.Root>
      <Menu.Trigger className={triggerClass(isDark)} aria-label={triggerLabel} title={triggerLabel}>
        <ShapeGlyph kind={active?.kind ?? 'rectangle'} size={16} />
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner align="center" side="bottom" sideOffset={8} style={{ zIndex: 50 }}>
          <Menu.Popup
            data-overlay-ui
            className="grid grid-cols-5 gap-0.5 rounded-[10px] border p-1"
            style={{
              background: 'var(--surface-popup)',
              borderColor: 'var(--surface-popup-border)',
              boxShadow: isDark
                ? '0 10px 8px -6px rgba(0,0,0,.58), 0 4px 16px 0 rgba(0,0,0,.5)'
                : '0 10px 8px -6px rgba(0,0,0,.18), 0 4px 16px 0 rgba(199,193,188,.5)',
            }}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {SHAPE_DEFS.map((def) => (
              <Menu.Item
                key={def.kind}
                closeOnClick
                aria-label={noun ? `Set ${noun} shape to ${def.label}` : def.label}
                title={def.label}
                onClick={() => onPick(def.kind)}
                className={cellClass(isDark, activeKind === def.kind)}
              >
                <ShapeGlyph kind={def.kind} size={18} />
              </Menu.Item>
            ))}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
