// Shared preset picker body: the sectioned viewport-preset list used by both
// the add-page tool popup and the inline page-size dropdown. Owners provide the
// surrounding frame/positioner; this just renders the rows.

import { VIEWPORT_PRESETS, deviceForPresetIndex } from '../../shared/device-catalog'

// Group presets into three device sections, keyed off catalog category.
function sectionIndex(category: string | undefined): number {
  if (category === 'iphone') return 0
  if (category === 'ipad') return 1
  return 2 // laptop / desktop
}

// iPhone rows drop the model year — the pixel size already distinguishes them
// (iPad/desktop numbers are physical sizes, so they stay).
function rowLabel(label: string): string {
  return label.replace('iPhone 14 ', 'iPhone ')
}

export function presetRowClass(isDark: boolean, active: boolean): string {
  const base =
    'flex w-full cursor-pointer items-center justify-between gap-4 rounded-[6px] px-2 py-1.5 text-left transition-colors'
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.1)] text-[var(--surface-panel-foreground)]`
      : `${base} bg-[var(--color-stone-200)] text-[var(--surface-panel-foreground)]`
  }
  return isDark
    ? `${base} text-[var(--surface-panel-foreground-muted)] hover:bg-[rgba(253,248,245,0.1)] hover:text-[var(--surface-panel-foreground)]`
    : `${base} text-[var(--surface-panel-foreground-muted)] hover:bg-[var(--color-stone-100)] hover:text-[var(--surface-panel-foreground)]`
}

function PresetRow({
  isDark,
  active,
  label,
  dims,
  ariaLabel,
  onClick,
}: {
  isDark: boolean
  active: boolean
  label: string
  dims?: string
  ariaLabel: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      className={presetRowClass(isDark, active)}
      onClick={onClick}
      aria-pressed={active}
      aria-label={ariaLabel}
    >
      <span className="truncate text-xs font-medium leading-none">{label}</span>
      <span className="whitespace-nowrap text-xs font-normal leading-none tabular-nums text-[var(--surface-panel-foreground-muted)]">
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

export function PresetList({
  isDark,
  activePreset,
  customActive,
  onSelectPreset,
  onSelectCustom,
  ariaVerb = 'Select',
  hideCustom = false,
}: {
  isDark: boolean
  activePreset: number | null
  customActive: boolean
  onSelectPreset: (index: number) => void
  onSelectCustom: () => void
  ariaVerb?: string
  // Batch selections have no single custom target, so the owner can drop the row.
  hideCustom?: boolean
}) {
  const sections: number[][] = [[], [], []]
  VIEWPORT_PRESETS.forEach((_, index) => {
    sections[sectionIndex(deviceForPresetIndex(index)?.category)].push(index)
  })

  return (
    <div className="flex w-56 flex-col">
      {sections.map((rows, sectionI) => (
        <div key={sectionI} className="flex flex-col">
          {sectionI > 0 && <HDivider isDark={isDark} />}
          {rows.map((index) => {
            const preset = VIEWPORT_PRESETS[index]
            const label = rowLabel(preset.label)
            return (
              <PresetRow
                key={preset.label}
                isDark={isDark}
                active={activePreset === index}
                label={label}
                dims={`${preset.width}×${preset.height}`}
                ariaLabel={`${ariaVerb} ${label}`}
                onClick={() => onSelectPreset(index)}
              />
            )
          })}
        </div>
      ))}
      {!hideCustom && (
        <>
          <HDivider isDark={isDark} />
          <PresetRow
            isDark={isDark}
            active={customActive}
            label="Custom"
            ariaLabel={`${ariaVerb} custom`}
            onClick={onSelectCustom}
          />
        </>
      )}
    </div>
  )
}
