// Selection popup for a connection edge (ADR 0008 family). Edges aren't scene
// entities, so this doesn't ride the SELECTION_POPUPS table — App mounts it
// directly off the single selected edge. Anchor math mirrors EdgeLayer: anchor
// points arrive in overlay coords (y already offset by canvasOrigin.y).

import { useEffect, useRef, useState } from 'react'
import { ArrowLeft, ArrowRight, Trash2, type LucideIcon } from 'lucide-react'
import type {
  CanvasSceneEntity,
  LayoutUpdateData,
  WorkspaceEdge,
} from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { slotForStorage } from '../../shared/canvas-colors'
import { resolveEdgeAnchors } from '../../shared/edge-geometry'
import { canvasToScreenPoint } from '../../shared/coords'
import { RoutingDropdown } from './RoutingDropdown'
import { CanvasItemPopup } from './CanvasItemPopup'
import { ColorDropdown } from './ColorDropdown'
import { EdgeStrokeDropdown } from './EdgeStrokeDropdown'

const POPUP_OFFSET_Y = 12

function EndpointToggle({
  isDark,
  active,
  label,
  Icon,
  onClick,
}: {
  isDark: boolean
  active: boolean
  label: string
  Icon: LucideIcon
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={active}
      title={label}
      className={`flex h-6 w-6 items-center justify-center rounded-[6px] border-0 bg-transparent transition-[background-color,color,opacity] ${
        active ? 'opacity-100' : 'opacity-45 hover:opacity-100'
      } ${
        isDark
          ? 'text-[var(--surface-foreground)] hover:bg-[rgba(253,248,245,0.1)]'
          : 'text-[var(--surface-foreground)] hover:bg-[var(--color-stone-100)]'
      }`}
      onClick={onClick}
    >
      <Icon size={14} />
    </button>
  )
}

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
  const entityMap = new Map<string, CanvasSceneEntity>()
  for (const e of entities) entityMap.set(e.id, e)
  const anchors = edge
    ? resolveEdgeAnchors(edge, entityMap, layout.zoom, layout.canvasOrigin.y, (point) =>
        canvasToScreenPoint(layout, point),
      )
    : null
  if (!edge || !anchors) return null

  const { from, to } = anchors
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
        <RoutingDropdown
          isDark={isDark}
          routing={edge.routing ?? 'bezier'}
          noun="edge"
          onPick={(routing) => api.updateEdge(edge.id, { routing })}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <EdgeStrokeDropdown
          isDark={isDark}
          lineStyle={edge.lineStyle ?? 'solid'}
          strokeWidth={edge.strokeWidth ?? 1.5}
          onSetStyle={(lineStyle) => api.updateEdge(edge.id, { lineStyle })}
          onSetWidth={(strokeWidth) => api.updateEdge(edge.id, { strokeWidth })}
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <EndpointToggle
          isDark={isDark}
          active={fromEnd === 'arrow'}
          label="Toggle start arrowhead"
          Icon={ArrowLeft}
          onClick={() =>
            api.updateEdge(edge.id, { fromEnd: fromEnd === 'arrow' ? 'none' : 'arrow' })
          }
        />
        <EndpointToggle
          isDark={isDark}
          active={toEnd === 'arrow'}
          label="Toggle end arrowhead"
          Icon={ArrowRight}
          onClick={() =>
            api.updateEdge(edge.id, { toEnd: toEnd === 'arrow' ? 'none' : 'arrow' })
          }
        />
        <CanvasItemPopup.Divider isDark={isDark} />
        <input
          type="text"
          className="h-6 w-28 rounded-[6px] border-0 bg-transparent px-1.5 text-[12px] outline-none text-[var(--surface-foreground)] placeholder:text-[var(--surface-foreground-muted)]"
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
