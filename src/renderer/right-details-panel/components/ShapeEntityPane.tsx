import { Trash2 } from 'lucide-react'
import type { PanelShapeEntityDetail } from '../../../shared/types'
import { SHAPE_DEFS, shapeDef } from '../../../shared/shapes'
import { ShapeGlyph } from '../../shared/ShapeGlyph'
import { mutedClass, paneDeleteBtnClass } from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { usePaneTheme } from '../PaneContext'
import { PaneField, PaneSection } from './PaneSection'
import { ColorSwatchPicker } from './ColorSwatchPicker'
import { PaneHeader } from './PaneHeader'

const STROKE_WIDTHS: number[] = [1, 2, 3, 4]

export function ShapeEntityPane({ shapeEntity }: { shapeEntity: PanelShapeEntityDetail }) {
  const isDark = usePaneTheme()
  const muted = mutedClass(isDark)
  const segmentBtn = (active: boolean) =>
    `flex items-center gap-1 rounded px-2 py-1 text-[11px] ${
      active
        ? isDark
          ? 'bg-zinc-700 text-zinc-100'
          : 'bg-zinc-200 text-zinc-900'
        : isDark
          ? 'text-zinc-300 hover:bg-zinc-800'
          : 'text-zinc-600 hover:bg-zinc-100'
    }`

  const currentStroke = shapeEntity.strokeWidth ?? 2

  return (
    <div className="flex flex-col">
      <PaneHeader
        icon={<ShapeGlyph kind={shapeEntity.shapeKind} size={14} />}
        label={shapeEntity.text.slice(0, 40) || shapeDef(shapeEntity.shapeKind).label}
        actions={
          <button
            type="button"
            className={paneDeleteBtnClass(isDark)}
            onClick={() => rightDetailsPanelApi.deleteShapeEntity(shapeEntity.id)}
            title="Delete"
            aria-label="Delete Shape"
          >
            <Trash2 size={14} />
          </button>
        }
      />

      <div className="px-2 pt-2 pb-2">
        <div className={`mb-1 text-[10px] font-medium ${muted}`}>shape</div>
        <div className="grid grid-cols-6 gap-1">
          {SHAPE_DEFS.map((def) => (
            <button
              key={def.kind}
              type="button"
              className={`flex h-7 items-center justify-center rounded ${segmentBtn(shapeEntity.shapeKind === def.kind)}`}
              onClick={() => rightDetailsPanelApi.updateEntity('shape', shapeEntity.id, { shapeKind: def.kind })}
              title={def.label}
              aria-label={def.label}
            >
              <ShapeGlyph kind={def.kind} size={16} />
            </button>
          ))}
        </div>
      </div>

      <PaneSection.Root>
        <PaneSection.Label>color</PaneSection.Label>
        <ColorSwatchPicker
          activeColor={shapeEntity.color ?? null}
          isDark={isDark}
          allowNone
          palette="soft"
          onSelectColor={(color) => rightDetailsPanelApi.updateEntity('shape', shapeEntity.id, { color })}
        />
      </PaneSection.Root>

      <PaneSection.Root>
        <PaneSection.Label>stroke</PaneSection.Label>
        <div className="flex items-center gap-1">
          {STROKE_WIDTHS.map((value) => (
            <button
              key={value}
              type="button"
              className={segmentBtn(currentStroke === value)}
              onClick={() => rightDetailsPanelApi.updateEntity('shape', shapeEntity.id, { strokeWidth: value })}
            >
              <span>{value}</span>
            </button>
          ))}
        </div>
      </PaneSection.Root>

      {shapeEntity.text ? (
        <PaneField label="content">
          <div className={`rounded px-2 py-1.5 text-[11px] leading-5 ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}>
            {shapeEntity.text}
          </div>
        </PaneField>
      ) : null}
    </div>
  )
}
