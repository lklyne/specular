// ADR 0008 — border control for the shape selection popup.
//
// One trigger (a stacked-lines glyph + chevron) opens a popup holding every
// border control: line style (solid / dashed / none), thickness, and an
// independent border color row — the FigJam layout. base-ui's Menu gives
// outside-click / Escape dismiss and positioning.

import { Menu } from '@base-ui/react/menu'
import { Ban, ChevronDown } from 'lucide-react'
import {
  paletteSlots,
  resolveCanvasColor,
  type CanvasColorSlot,
  type CanvasPalette,
} from '../../shared/canvas-colors'
import type { ShapeBorderStyle } from '../../shared/types'
import { swatchDotShadow, swatchRingShadow } from './colorSwatchStyle'

/** Thickness presets, mirroring the right panel's stroke widths. */
const BORDER_WIDTH_PRESETS: readonly number[] = [1, 2, 3, 4]

const STYLE_OPTIONS: { style: ShapeBorderStyle; label: string }[] = [
  { style: 'solid', label: 'Solid' },
  { style: 'dashed', label: 'Dashed' },
  { style: 'none', label: 'None' },
]

/** The border glyph, exported verbatim from Figma (node 527:63), themed via
 *  currentColor so it tracks the trigger's text color. */
function BorderGlyph({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 14 14" fill="none" aria-hidden>
      <path
        d="M12.25 2.41699C12.5261 2.41699 12.75 2.64085 12.75 2.91699C12.7499 3.19305 12.5261 3.41699 12.25 3.41699H1.75C1.47392 3.41699 1.2501 3.19305 1.25 2.91699C1.25 2.64085 1.47386 2.41699 1.75 2.41699H12.25Z"
        fill="currentColor"
      />
      <rect x="1.5" y="5" width="11" height="2" rx="0.5" fill="none" stroke="currentColor" />
      <rect x="1.5" y="9" width="11" height="3" rx="0.5" fill="none" stroke="currentColor" />
    </svg>
  )
}

function LineGlyph({ dashed }: { dashed: boolean }) {
  return (
    <svg width={14} height={14} viewBox="0 0 14 14" aria-hidden>
      <line
        x1={1}
        y1={7}
        x2={13}
        y2={7}
        stroke="currentColor"
        strokeWidth={2}
        strokeLinecap="round"
        strokeDasharray={dashed ? '3 2.5' : undefined}
      />
    </svg>
  )
}

function triggerClass(isDark: boolean): string {
  const base =
    'flex h-6 items-center gap-1 rounded-[6px] border-0 pl-1.5 pr-1 transition-colors'
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100 data-[popup-open]:bg-[rgba(253,248,245,0.1)] data-[popup-open]:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900 data-[popup-open]:bg-[var(--color-stone-200)] data-[popup-open]:text-zinc-900`
}

function segmentClass(isDark: boolean, active: boolean): string {
  const base =
    'flex flex-1 items-center justify-center gap-1 rounded-[6px] px-2 py-1 text-xs leading-none transition-colors'
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.1)] text-zinc-100`
      : `${base} bg-[var(--color-stone-200)] text-zinc-900`
  }
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.08)] hover:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900`
}

export function BorderDropdown({
  isDark,
  borderStyle,
  strokeWidth,
  activeColorSlot,
  palette,
  noun,
  onSetStyle,
  onSetWidth,
  onSetColor,
}: {
  isDark: boolean
  /** Current style, or null when the selection is mixed. */
  borderStyle: ShapeBorderStyle | null
  /** Current thickness, or null when mixed. */
  strokeWidth: number | null
  activeColorSlot: CanvasColorSlot | null
  palette: CanvasPalette
  noun?: string
  onSetStyle: (style: ShapeBorderStyle) => void
  onSetWidth: (width: number) => void
  onSetColor: (storage: string) => void
}) {
  const colorDisabled = borderStyle === 'none'
  return (
    <Menu.Root>
      <Menu.Trigger className={triggerClass(isDark)} aria-label="Border" title="Border">
        <BorderGlyph size={14} />
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner align="center" side="bottom" sideOffset={8} style={{ zIndex: 50 }}>
          <Menu.Popup
            data-overlay-ui
            className="flex w-[220px] flex-col gap-1 rounded-[10px] border p-1"
            style={{
              background: 'var(--surface-popup)',
              borderColor: 'var(--surface-popup-border)',
              boxShadow: isDark
                ? '0 10px 8px -6px rgba(0,0,0,.58), 0 4px 16px 0 rgba(0,0,0,.5)'
                : '0 10px 8px -6px rgba(0,0,0,.18), 0 4px 16px 0 rgba(199,193,188,.5)',
            }}
            onPointerDown={(event) => event.stopPropagation()}
          >
            <div className="flex items-center gap-1">
              {STYLE_OPTIONS.map(({ style, label }) => (
                <button
                  key={style}
                  type="button"
                  className={segmentClass(isDark, borderStyle === style)}
                  aria-pressed={borderStyle === style}
                  onClick={() => onSetStyle(style)}
                >
                  {style === 'none' ? <Ban size={13} /> : <LineGlyph dashed={style === 'dashed'} />}
                  <span>{label}</span>
                </button>
              ))}
            </div>
            <div className="flex items-center gap-1">
              {BORDER_WIDTH_PRESETS.map((width) => (
                <button
                  key={width}
                  type="button"
                  disabled={colorDisabled}
                  className={`${segmentClass(isDark, strokeWidth === width && !colorDisabled)} disabled:opacity-30 disabled:hover:bg-transparent`}
                  aria-label={`Set border width to ${width}px`}
                  aria-pressed={strokeWidth === width}
                  onClick={() => onSetWidth(width)}
                >
                  {width}
                </button>
              ))}
            </div>
            <div className={`flex items-center justify-between gap-1 px-0.5 ${colorDisabled ? 'pointer-events-none opacity-30' : ''}`}>
              {paletteSlots(palette).map((slot) => {
                const swatch = slot.hex ?? resolveCanvasColor(slot.storage, { role: 'fill', isDark })
                const label = noun
                  ? `Set ${noun} border color to ${slot.label}`
                  : `Set border color to ${slot.label}`
                return (
                  <button
                    key={slot.id}
                    type="button"
                    aria-label={label}
                    className="flex h-5 w-5 items-center justify-center rounded-full transition-shadow bg-[var(--surface-popup)]"
                    style={{ boxShadow: swatchRingShadow(swatch, activeColorSlot === slot.id, isDark) }}
                    onClick={() => onSetColor(slot.storage)}
                  >
                    <span
                      className="block h-3 w-3 rounded-full"
                      style={{ background: swatch, boxShadow: swatchDotShadow(isDark) }}
                    />
                  </button>
                )
              })}
            </div>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
