import { Group } from 'lucide-react'
import type { PanelGroupEntityDetail } from '../../../shared/types'
import { resolveCanvasColor } from '../../../shared/canvas-colors'
import { mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'
import { PaneField } from './PaneSection'
import { PaneHeader } from './PaneHeader'

export function GroupEntityPane({ groupEntity }: { groupEntity: PanelGroupEntityDetail }) {
  const isDark = usePaneTheme()
  // Groups paint in the vivid palette (ADR 0013 §1).
  const groupSwatch = groupEntity.color
    ? resolveCanvasColor(groupEntity.color, { palette: 'vivid', isDark })
    : null

  return (
    <div className="flex flex-col">
      <PaneHeader
        icon={<Group size={14} className="shrink-0 text-zinc-500" />}
        label={groupEntity.label || 'Group'}
      />

      {groupSwatch ? (
        <PaneField label="Color">
          <div className="flex items-center gap-2">
            <div
              className="size-4 shrink-0 rounded border border-zinc-300 dark:border-zinc-600"
              style={{ backgroundColor: groupSwatch }}
            />
            <span className="text-[11px]">{groupSwatch}</span>
          </div>
        </PaneField>
      ) : null}

      <PaneField label="Members">
        <div className={`text-[11px] ${mutedClass(isDark)}`}>
          {groupEntity.entityIds.length} {groupEntity.entityIds.length === 1 ? 'entity' : 'entities'}
        </div>
      </PaneField>
    </div>
  )
}
