import { Layers } from 'lucide-react'
import type { PageColorScheme, PanelMultiEntitySummary } from '../../../shared/types'
import { dividerClass, mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { PaneSection } from './PaneSection'
import { PaneHeader } from './PaneHeader'

type ColorSchemeChoice = 'system' | PageColorScheme

const COLOR_SCHEME_OPTIONS: { value: ColorSchemeChoice; label: string }[] = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
]

export function MultiEntityPane({ multiEntities }: { multiEntities: PanelMultiEntitySummary[] }) {
  const isDark = usePaneTheme()
  const muted = mutedClass(isDark)
  const divider = dividerClass(isDark)

  const selectedPageIds = multiEntities.filter((e) => e.kind === 'page').map((e) => e.id)
  const schemeKeys = new Set(
    multiEntities.filter((e) => e.kind === 'page').map((e) => e.colorScheme ?? 'system'),
  )
  // Mixed values across the selected pages render every segment inactive
  // (indeterminate) rather than picking one.
  const activeScheme: ColorSchemeChoice | null = schemeKeys.size === 1 ? [...schemeKeys][0] : null

  const setColorScheme = (choice: ColorSchemeChoice) => {
    const colorScheme = choice === 'system' ? null : choice
    for (const id of selectedPageIds) rightDetailsPanelApi.setPageColorScheme(id, colorScheme)
  }

  const tabBg = 'bg-[var(--surface-interactive)] border border-[var(--surface-input-border)]'
  const tabActive = isDark
    ? 'bg-[var(--surface-toolbar)] text-zinc-100'
    : 'bg-[var(--surface-input)] text-zinc-800 shadow-sm'
  const tabInactive = isDark
    ? 'text-zinc-500 hover:text-zinc-300'
    : 'text-zinc-400 hover:text-zinc-600'

  return (
    <div className="flex flex-col">
      <PaneHeader
        icon={<Layers size={14} className="shrink-0 text-zinc-500" />}
        label={`${multiEntities.length} items selected`}
      />

      <PaneSection.Root>
        <div className="flex flex-col gap-1">
          {multiEntities.map((entity) => (
            <div
              key={entity.id}
              className={`flex items-center gap-2 rounded px-2 py-1 text-[11px] ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}
            >
              <span className={`shrink-0 text-[10px] ${muted}`}>{entity.kind}</span>
              <span className="min-w-0 truncate">{entity.label}</span>
            </div>
          ))}
        </div>
      </PaneSection.Root>

      {selectedPageIds.length >= 2 ? (
        <section className={`border-t ${divider}`}>
          <div className="flex items-center justify-between px-2 py-2">
            <span className={`text-[11px] ${muted}`}>Color scheme</span>
            <div className={`flex rounded-md ${tabBg} p-0.5`}>
              {COLOR_SCHEME_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  type="button"
                  className={`rounded px-2 py-1 text-[11px] transition-colors ${
                    activeScheme === opt.value ? tabActive : tabInactive
                  }`}
                  aria-pressed={activeScheme === opt.value}
                  onClick={() => setColorScheme(opt.value)}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>
        </section>
      ) : null}
    </div>
  )
}
