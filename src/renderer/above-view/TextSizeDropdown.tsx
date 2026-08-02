// ADR 0013 §2 — labeled-dropdown for per-entity text size.
//
// Trigger shows the preset label that matches the current value (or "Custom"
// if it doesn't match a preset). Opening the dropdown lists the five presets
// with a ✓ marker on the active one, plus a raw-pixel input at the bottom
// (commits on Enter or blur; clamps to 8–256 silently).
//
// Outside-click and Escape dismiss via base-ui's Menu primitive. Preset clicks
// commit immediately.

import { Menu } from '@base-ui/react/menu'
import { Check, ChevronDown } from 'lucide-react'
import { useState } from 'react'

const TEXT_SIZE_PRESETS = [
  { label: 'Small', value: 14 },
  { label: 'Medium', value: 32 },
  { label: 'Large', value: 56 },
  { label: 'Extra large', value: 96 },
  { label: 'Huge', value: 144 },
] as const

export const TEXT_SIZE_MIN = 8
export const TEXT_SIZE_MAX = 256
export const TEXT_SIZE_DEFAULT = 14

export function presetLabelForValue(value: number): string {
  const match = TEXT_SIZE_PRESETS.find((p) => p.value === value)
  return match ? match.label : 'Custom'
}

export function clampTextSize(value: number): number {
  if (!Number.isFinite(value)) return TEXT_SIZE_DEFAULT
  return Math.min(TEXT_SIZE_MAX, Math.max(TEXT_SIZE_MIN, Math.round(value)))
}

/**
 * Line-height multiplier for a given text size. Large display text needs
 * tighter leading than small body text — a constant ratio reads too airy
 * as size grows. Eases linearly from 1.5 at the Small preset (14px) down to
 * 1.1 at Extra large (96px) and holds there for Huge / custom values.
 */
export function lineHeightForTextSize(px: number): number {
  const progress = (px - 14) / (96 - 14)
  const ratio = 1.5 + progress * (1.1 - 1.5)
  return Math.min(1.5, Math.max(1.1, ratio))
}

function popupClass(isDark: boolean): string {
  const base = 'z-50 min-w-[140px] rounded-[10px] border p-1 shadow-xl outline-none'
  return isDark
    ? `${base} border-zinc-700 bg-zinc-900 text-[var(--surface-panel-foreground)]`
    : `${base} border-zinc-200 bg-white text-[var(--surface-panel-foreground)]`
}

function itemClass(isDark: boolean): string {
  const base =
    'flex cursor-default items-center justify-between gap-3 rounded-[7px] px-2 py-1 text-xs outline-none'
  return isDark
    ? `${base} text-[var(--surface-panel-foreground)] data-[highlighted]:bg-zinc-800`
    : `${base} text-[var(--surface-panel-foreground)] data-[highlighted]:bg-zinc-100`
}

function triggerClass(isDark: boolean): string {
  const base =
    'flex h-6 items-center gap-1 rounded-[6px] border-0 px-1.5 text-xs leading-none transition-colors'
  return isDark
    ? `${base} text-[var(--surface-panel-foreground-muted)] hover:bg-[rgba(253,248,245,0.1)] hover:text-[var(--surface-panel-foreground)] data-[popup-open]:bg-[rgba(253,248,245,0.1)] data-[popup-open]:text-[var(--surface-panel-foreground)]`
    : `${base} text-[var(--surface-panel-foreground-muted)] hover:bg-[var(--color-stone-100)] hover:text-[var(--surface-panel-foreground)] data-[popup-open]:bg-[var(--color-stone-200)] data-[popup-open]:text-[var(--surface-panel-foreground)]`
}

function inputClass(isDark: boolean): string {
  const base =
    'h-6 w-full rounded-[6px] border bg-transparent px-2 text-xs outline-none'
  return isDark
    ? `${base} border-zinc-700 text-[var(--surface-panel-foreground)] focus:border-zinc-500`
    : `${base} border-zinc-300 text-[var(--surface-panel-foreground)] focus:border-zinc-500`
}

export function TextSizeDropdown({
  isDark,
  value,
  ariaLabel,
  onPick,
}: {
  isDark: boolean
  /** Current text size in px. */
  value: number
  ariaLabel: string
  onPick: (size: number) => void
}) {
  const label = presetLabelForValue(value)
  return (
    <Menu.Root>
      <Menu.Trigger
        className={triggerClass(isDark)}
        aria-label={ariaLabel}
        title={ariaLabel}
      >
        <span>{label}</span>
        <ChevronDown size={12} />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner align="start" sideOffset={6} style={{ zIndex: 50 }}>
          <Menu.Popup
            data-overlay-ui
            className={popupClass(isDark)}
            onPointerDown={(event) => event.stopPropagation()}
          >
            {TEXT_SIZE_PRESETS.map((preset) => {
              const active = preset.value === value
              return (
                <Menu.Item
                  key={preset.value}
                  className={itemClass(isDark)}
                  onClick={() => onPick(preset.value)}
                >
                  <span>{preset.label}</span>
                  <span className="flex w-3 items-center justify-center">
                    {active ? <Check size={12} /> : null}
                  </span>
                </Menu.Item>
              )
            })}
            <div className={`mx-1 my-1 h-px ${isDark ? 'bg-zinc-700' : 'bg-zinc-200'}`} />
            <div className="px-1 pb-1">
              <RawSizeInput isDark={isDark} value={value} onCommit={onPick} />
            </div>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}

function RawSizeInput({
  isDark,
  value,
  onCommit,
}: {
  isDark: boolean
  value: number
  onCommit: (size: number) => void
}) {
  const [draft, setDraft] = useState(String(value))
  const escapedRef = useState({ flag: false })[0]

  const commit = () => {
    if (escapedRef.flag) {
      // Escape always discards: reset to the active value, don't write.
      escapedRef.flag = false
      setDraft(String(value))
      return
    }
    const parsed = Number.parseInt(draft, 10)
    if (Number.isNaN(parsed)) {
      setDraft(String(value))
      return
    }
    const clamped = clampTextSize(parsed)
    setDraft(String(clamped))
    if (clamped !== value) onCommit(clamped)
  }

  return (
    <input
      type="text"
      inputMode="numeric"
      className={inputClass(isDark)}
      value={draft}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault()
          commit()
        } else if (event.key === 'Escape') {
          // Mark discard so the blur that closes the menu does not commit
          // the in-flight draft. ADR 0013 §2 — Escape dismiss without commit.
          escapedRef.flag = true
        }
      }}
      aria-label="Custom text size in pixels"
    />
  )
}
