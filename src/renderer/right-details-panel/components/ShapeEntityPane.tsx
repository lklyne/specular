import { Trash2 } from 'lucide-react'
import type { PanelShapeEntityDetail } from '../../../shared/types'
import { shapeDef } from '../../../shared/shapes'
import { ShapeGlyph } from '../../shared/ShapeGlyph'
import { paneDeleteBtnClass } from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { usePaneTheme } from '../PaneContext'
import { PaneField } from './PaneSection'
import { PaneHeader } from './PaneHeader'

export function ShapeEntityPane({
  shapeEntity,
}: {
  shapeEntity: PanelShapeEntityDetail
}) {
  const isDark = usePaneTheme()

  return (
    <div className="flex flex-col">
      <PaneHeader
        icon={<ShapeGlyph kind={shapeEntity.shapeKind} size={14} />}
        label={
          shapeEntity.text.slice(0, 40) || shapeDef(shapeEntity.shapeKind).label
        }
        actions={
          <button
            type="button"
            className={paneDeleteBtnClass(isDark)}
            onClick={() =>
              rightDetailsPanelApi.deleteShapeEntity(shapeEntity.id)
            }
            title="Delete"
            aria-label="Delete Shape"
          >
            <Trash2 size={14} />
          </button>
        }
      />

      {shapeEntity.text ? (
        <PaneField label="content">
          <div
            className={`rounded px-2 py-1.5 text-[11px] leading-5 ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}
          >
            {shapeEntity.text}
          </div>
        </PaneField>
      ) : null}
    </div>
  )
}
