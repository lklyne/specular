// Selection popup for a connection edge (ADR 0008 family). Edges aren't scene
// entities, so this doesn't ride the SELECTION_POPUPS table — App mounts it
// directly off the single selected edge. Anchor math mirrors EdgeLayer: anchor
// points arrive in overlay coords (y already offset by canvasOrigin.y).

import { useEffect, useRef, useState } from 'react'
import { ArrowLeftFromLine, ArrowRightToLine, Trash2 } from 'lucide-react'
import type {
  CanvasSceneEntity,
  LayoutUpdateData,
  WorkspaceEdge,
} from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { slotForStorage } from '../../shared/canvas-colors'
import { autoSides, getAnchorPoint } from '../../shared/edge-geometry'
import { CanvasItemPopup } from './CanvasItemPopup'
import { ColorDropdown } from './ColorDropdown'

const POPUP_OFFSET_Y = 12

export function EdgePopup({
  api,
  isDark,
  layout,
  edge,
}: {
  api: Pick<CanvasBgElectronAPI, 'updateEdge' | 'deleteEdge'>
  isDark: boolean
  layout: LayoutUpdateData
  edge: WorkspaceEdge | null
}) {
  const [labelDraft, setLabelDraft] = useState<string | null>(null)
  // Closing the popup (deselect / menu close) can unmount the input before its
  // onBlur fires, dropping the typed label. Hold the pending edit and flush it
  // on unmount so a closed menu still commits.
  const pending = useRef<{ id: string; label: string } | null>(null)
  useEffect(
    () => () => {
      const p = pending.current
      if (p) api.updateEdge(p.id, { label: p.label })
    },
    [api],
  )

  const entities = layout.entities
  const fromEntity = edge && findEntity(entities, edge.fromEntityId)
  const toEntity = edge && findEntity(entities, edge.toEntityId)
  if (!edge || !fromEntity || !toEntity) return null

  const { fromSide, toSide } =
    edge.fromSide && edge.toSide
      ? { fromSide: edge.fromSide, toSide: edge.toSide }
      : autoSides(fromEntity, toEntity)
  const from = getAnchorPoint(fromEntity, fromSide, layout.zoom, layout.canvasOrigin.y)
  const to = getAnchorPoint(toEntity, toSide, layout.zoom, layout.canvasOrigin.y)
  const mid = { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 }

  const fromEnd = edge.fromEnd ?? 'none'
  const toEnd = edge.toEnd ?? 'arrow'
  const labelValue = labelDraft ?? edge.label ?? ''
  const commitLabel = () => {
    api.updateEdge(edge.id, { label: labelValue })
    pending.current = null
    setLabelDraft(null)
  }

  return (
    <div
      data-overlay-ui
      className="pointer-events-auto absolute"
      style={{
        left: mid.x,
        top: mid.y,
        transform: `translate(-50%, calc(-100% - ${POPUP_OFFSET_Y}px))`,
        zIndex: 20,
      }}
    >
      <CanvasItemPopup.Frame isDark={isDark}>
        <ColorDropdown
          isDark={isDark}
          palette="vivid"
          role="ink"
          noun="edge"
          activeSlot={slotForStorage(edge.color ?? null)}
          onPick={(storage) => api.updateEdge(edge.id, { color: storage })}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.IconButton
          isDark={isDark}
          active={fromEnd === 'arrow'}
          title="Start arrowhead"
          ariaLabel="Toggle start arrowhead"
          onClick={() =>
            api.updateEdge(edge.id, { fromEnd: fromEnd === 'arrow' ? 'none' : 'arrow' })
          }
        >
          <ArrowLeftFromLine size={14} />
        </CanvasItemPopup.IconButton>
        <CanvasItemPopup.IconButton
          isDark={isDark}
          active={toEnd === 'arrow'}
          title="End arrowhead"
          ariaLabel="Toggle end arrowhead"
          onClick={() =>
            api.updateEdge(edge.id, { toEnd: toEnd === 'arrow' ? 'none' : 'arrow' })
          }
        >
          <ArrowRightToLine size={14} />
        </CanvasItemPopup.IconButton>
        <CanvasItemPopup.Divider isDark={isDark} />
        <input
          type="text"
          className={`h-6 w-28 rounded-[6px] border-0 bg-transparent px-1.5 text-[12px] outline-none ${
            isDark
              ? 'text-zinc-100 placeholder:text-zinc-500'
              : 'text-zinc-900 placeholder:text-zinc-400'
          }`}
          placeholder="Label…"
          value={labelValue}
          onChange={(e) => {
            setLabelDraft(e.target.value)
            pending.current = { id: edge.id, label: e.target.value }
          }}
          onBlur={commitLabel}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              commitLabel()
              ;(e.target as HTMLInputElement).blur()
            }
          }}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <CanvasItemPopup.IconButton
          isDark={isDark}
          title="Delete edge"
          ariaLabel="Delete edge"
          onClick={() => api.deleteEdge(edge.id)}
        >
          <Trash2 size={14} />
        </CanvasItemPopup.IconButton>
      </CanvasItemPopup.Frame>
    </div>
  )
}

function findEntity(
  entities: CanvasSceneEntity[],
  id: string,
): CanvasSceneEntity | undefined {
  return entities.find((entity) => entity.id === id)
}
