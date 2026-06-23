import { Layers } from 'lucide-react'
import type { PanelMultiEntitySummary } from '../../../shared/types'
import { mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'
import { PaneSection } from './PaneSection'
import { PaneHeader } from './PaneHeader'

export function MultiEntityPane({ multiEntities }: { multiEntities: PanelMultiEntitySummary[] }) {
  const isDark = usePaneTheme()
  const muted = mutedClass(isDark)

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
    </div>
  )
}
