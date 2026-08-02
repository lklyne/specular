import { PenLine, Trash2 } from 'lucide-react'
import type { PanelDrawingEntityDetail } from '../../../shared/types'
import { paneDeleteBtnClass } from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { usePaneTheme } from '../PaneContext'
import { PaneField } from './PaneSection'
import { PaneHeader } from './PaneHeader'

export function DrawingEntityPane({ drawingEntity }: { drawingEntity: PanelDrawingEntityDetail }) {
  const isDark = usePaneTheme()

  return (
    <div className="flex flex-col">
      <PaneHeader
        icon={<PenLine size={14} className="shrink-0 text-[var(--surface-panel-foreground-muted)]" />}
        label={`Drawing (${drawingEntity.strokeCount} stroke${drawingEntity.strokeCount === 1 ? '' : 's'})`}
        actions={
          <button
            type="button"
            className={paneDeleteBtnClass(isDark)}
            onClick={() => rightDetailsPanelApi.deleteDrawingEntity(drawingEntity.id)}
            title="Delete"
            aria-label="Delete Drawing"
          >
            <Trash2 size={14} />
          </button>
        }
      />

      <PaneField label="Bounds">
        <div className={`rounded px-2 py-1.5 text-[11px] ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}>
          {drawingEntity.width} x {drawingEntity.height}
        </div>
      </PaneField>

      <PaneField label="Strokes">
        <div className={`rounded px-2 py-1.5 text-[11px] ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}>
          {drawingEntity.strokeCount}
        </div>
      </PaneField>
    </div>
  )
}
