// ADR 0008 §1 — add-page tool popup. Unlike the other tool popups it carries no
// tool defaults: the chosen preset rides on the active `add-page` Tool object
// (tool.presetIndex / tool.customSize), so picking here just re-arms the tool.
//
// Self-contained on purpose — the preset-row layout is meant to be iterated on
// without disturbing the shared CanvasItemPopup primitives.

import type { ComponentType } from 'react'
import { Laptop, Monitor, Smartphone, SquareDashed, Tablet } from 'lucide-react'
import { VIEWPORT_PRESETS, deviceForPresetIndex } from '../../shared/device-catalog'
import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { CanvasItemPopup } from './CanvasItemPopup'

const CATEGORY_ICON: Record<string, ComponentType<{ size?: number }>> = {
  iphone: Smartphone,
  ipad: Tablet,
  laptop: Laptop,
  desktop: Monitor,
}

// Group presets into three device sections, keyed off catalog category.
function sectionIndex(category: string | undefined): number {
  if (category === 'iphone') return 0
  if (category === 'ipad') return 1
  return 2 // laptop / desktop
}

function rowClass(isDark: boolean, active: boolean): string {
  const base =
    'flex w-full items-center justify-between gap-4 rounded-[6px] px-2 py-1 text-left transition-colors'
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.1)] text-zinc-100`
      : `${base} bg-[var(--color-stone-200)] text-zinc-900`
  }
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[var(--color-stone-100)] hover:text-zinc-900`
}

function PresetRow({
  isDark,
  active,
  label,
  dims,
  Icon,
  onClick,
}: {
  isDark: boolean
  active: boolean
  label: string
  dims?: string
  Icon: ComponentType<{ size?: number }>
  onClick: () => void
}) {
  return (
    <button
      type="button"
      className={rowClass(isDark, active)}
      onClick={onClick}
      aria-pressed={active}
      aria-label={`Add ${label} page`}
    >
      {/* Icons hidden for now — re-enable to bring them back. */}
      {/* <Icon size={14} /> */}
      <span className="truncate text-xs font-medium leading-none">{label}</span>
      <span className="whitespace-nowrap text-xs font-normal leading-none tabular-nums text-zinc-500">
        {dims ?? ''}
      </span>
    </button>
  )
}

// Same style as CanvasItemPopup.Divider, rotated for the vertical list.
function HDivider({ isDark }: { isDark: boolean }) {
  return (
    <div
      aria-hidden
      className={`my-1 h-px w-full ${isDark ? 'bg-white/20' : 'bg-zinc-900/20'}`}
    />
  )
}

// iPhone rows drop the model year — the pixel size already distinguishes them
// (iPad/desktop numbers are physical sizes, so they stay).
function rowLabel(label: string): string {
  return label.replace('iPhone 14 ', 'iPhone ')
}

export function PageToolPopup({
  api,
  isDark,
  layout,
}: {
  api: Pick<CanvasBgElectronAPI, 'setTool'>
  isDark: boolean
  layout: LayoutUpdateData
}) {
  const tool = layout.activeTool
  const customActive = tool.kind === 'add-page' && tool.customSize === true
  const activePreset =
    tool.kind === 'add-page' && !tool.customSize ? tool.presetIndex ?? 0 : null

  const sections: { index: number; category: string | undefined }[][] = [[], [], []]
  VIEWPORT_PRESETS.forEach((_, index) => {
    const category = deviceForPresetIndex(index)?.category
    sections[sectionIndex(category)].push({ index, category })
  })

  return (
    <CanvasItemPopup.ViewportAnchor layout={layout} open offset={8}>
      <CanvasItemPopup.Frame isDark={isDark}>
        <div className="flex w-56 flex-col">
          {sections.map((rows, sectionI) => (
            <div key={sectionI} className="flex flex-col">
              {sectionI > 0 && <HDivider isDark={isDark} />}
              {rows.map(({ index, category }) => {
                const preset = VIEWPORT_PRESETS[index]
                return (
                  <PresetRow
                    key={preset.label}
                    isDark={isDark}
                    active={activePreset === index}
                    label={rowLabel(preset.label)}
                    dims={`${preset.width}×${preset.height}`}
                    Icon={CATEGORY_ICON[category ?? ''] ?? Monitor}
                    onClick={() => api.setTool({ kind: 'add-page', presetIndex: index })}
                  />
                )
              })}
            </div>
          ))}
          <HDivider isDark={isDark} />
          <PresetRow
            isDark={isDark}
            active={customActive}
            label="Custom"
            Icon={SquareDashed}
            onClick={() => api.setTool({ kind: 'add-page', customSize: true })}
          />
        </div>
      </CanvasItemPopup.Frame>
    </CanvasItemPopup.ViewportAnchor>
  )
}
