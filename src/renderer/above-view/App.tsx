import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type {
  CanvasBgElectronAPI,
  CanvasSceneEntity,
  CanvasSceneDrawingEntity,
  CanvasSceneFileEntity,
  CanvasScenePageEntity,
  CanvasSceneShapeEntity,
  CanvasSceneTextEntity,
  LayoutUpdateData,
  SelectionOverlayPayload,
  ThemeData,
  WorkspaceEdge,
} from '../../shared/types'
import type { CanvasGuidesPayload } from '../../shared/canvas-guides'
import {
  canvasToScreenX,
  canvasToScreenY,
  isOverlayUiTarget,
  normalizeRect,
  screenPointToCanvasPoint,
  screenRectToCanvasRect,
  snapToGrid,
  squareConstrainedRect,
} from '../../shared/gesture-utils'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import { isAnnotationTool, toolHasPopup } from '../../shared/tool'
import { DRAW_CURSOR, selectionColor } from '../canvas-bg/canvasBgConstants'
import { ActivePageHighlightLayer } from '../canvas-bg/AgentCursorLayer'
import { PlacementPreviewLayer } from '../canvas-bg/CanvasGridSurface'
import { buildPendingPlacementPreview } from '../canvas-bg/canvasBgSelectors'
import { DrawingLayer, SavedDrawingEntities } from './DrawingsLayer'
import { FileBodyLayer, type FileJsonModeMap } from './FileBodyLayer'
import { PageFocusRingLayer } from './PageFocusRingLayer'
import { GroupBoundsLayer } from './GroupBoundsLayer'
import { SelectionOutlineLayer } from './SelectionOutlineLayer'
import { ShapeBodyLayer } from './ShapeBodyLayer'
import { StickyBodyLayer } from './StickyBodyLayer'
import { RegionSelectAnnotations } from './AnnotationsLayer'
import {
  AnnotationThreadPopover,
  PendingAnnotationComposer,
  PendingElementOutline,
} from './CommentsLayer'
import { MarqueeLayer } from './MarqueeLayer'
import { useAnnotationDrawingGestures } from './useAnnotationDrawingGestures'
import { useAnnotationDraftState } from './useAnnotationDraftState'
import { useAnnotationThreadState, annotationThreadPosition } from './useAnnotationThreadState'
import { useCommentToolPointerBroadcast } from './useCommentToolPointerBroadcast'
import { useLiveAnnotationBboxes } from './useLiveAnnotationBboxes'
import { canvasRectToScreenRect, pendingElementComposerPosition } from './annotationMath'
import {
  FULL_ROUTER_CONSUME,
  useCanvasPointerRouter,
} from './useCanvasPointerRouter'
import { EdgeDragLayer } from './EdgeDragLayer'
import { EdgeLayer } from './EdgeLayer'
import { PageChromeOverlay } from './PageChrome'
import { PagePopup } from './PagePopup'
import { FilePopup } from './FilePopup'
import { FileChromeOverlay } from './FileChrome'
import { GroupRenameOverlay } from './GroupRenameLabel'
import { DrawingPopup } from './DrawingPopup'
import { DrawToolPopup } from './DrawToolPopup'
import { GroupPopup } from './GroupPopup'
import { ShapePopup } from './ShapePopup'
import { ShapeToolPopup } from './ShapeToolPopup'
import { StickyNotePopover } from './StickyNotePopover'
import { TextToolPopup } from './TextToolPopup'
import { EDGE_DRAG_IDLE, type EdgeDragState } from '../../shared/edge-drag-controller'
import type { DragCopyPreviewBox } from './optionDragCopy'
import { useCanvasClipboard } from '../canvas-bg/useCanvasClipboard'
import { buildAboveViewHandlers } from './binding-handlers'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useRendererBindingHandlers } from '../shared/hooks/useRendererBindingHandlers'
import { useTheme } from '../shared/hooks/useTheme'
import { useViewportWheelAndMiddlePan } from '../shared/hooks/useViewportWheelAndMiddlePan'

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI
const MIN_SHAPE_DRAG_SIZE = 24

function DragCopyPreviewLayer({
  previews,
  isDark,
}: {
  previews: DragCopyPreviewBox[]
  isDark: boolean
}) {
  return (
    <>
      {previews.map((preview) => (
        <div
          key={`drag-copy-preview-${preview.id}`}
          className="pointer-events-none absolute rounded-[8px] border"
          style={{
            left: preview.left,
            top: preview.top,
            width: preview.width,
            height: preview.height,
            background: isDark ? 'rgba(244, 244, 245, 0.14)' : 'rgba(39, 39, 42, 0.08)',
            borderColor: isDark ? 'rgba(244, 244, 245, 0.6)' : 'rgba(39, 39, 42, 0.42)',
            boxShadow: isDark
              ? '0 10px 30px rgba(0, 0, 0, 0.28)'
              : '0 10px 30px rgba(24, 24, 27, 0.12)',
          }}
        />
      ))}
    </>
  )
}

function GuideOverlayLayer({
  guides,
  layoutData,
  isDark,
}: {
  guides: CanvasGuidesPayload
  layoutData: LayoutUpdateData
  isDark: boolean
}) {
  if (guides.alignmentGuides.length === 0 && guides.distributionGuides.length === 0) return null

  const color = selectionColor(isDark)
  const toScreenX = (x: number) => x * layoutData.zoom + layoutData.pan.x + layoutData.canvasOrigin.x
  const toOverlayY = (y: number) => y * layoutData.zoom + layoutData.pan.y
  const distributionColor = '#EC4899'
  const distributionCapHalf = 9
  const distributionCapInset = 1

  return (
    <svg
      className="pointer-events-none absolute inset-0 h-full w-full overflow-visible"
      aria-hidden="true"
    >
      {guides.alignmentGuides.map((guide, index) => (
        guide.axis === 'horizontal' ? (
          <line
            key={`${guide.draggedId}-${guide.candidateId}-${guide.draggedReference}-${guide.candidateReference}-${index}`}
            x1={toScreenX(guide.start)}
            y1={toOverlayY(guide.coordinate)}
            x2={toScreenX(guide.end)}
            y2={toOverlayY(guide.coordinate)}
            stroke={color}
            strokeWidth={1}
            vectorEffect="non-scaling-stroke"
          />
        ) : (
          <line
            key={`${guide.draggedId}-${guide.candidateId}-${guide.draggedReference}-${guide.candidateReference}-${index}`}
            x1={toScreenX(guide.coordinate)}
            y1={toOverlayY(guide.start)}
            x2={toScreenX(guide.coordinate)}
            y2={toOverlayY(guide.end)}
            stroke={color}
            strokeWidth={1}
            vectorEffect="non-scaling-stroke"
          />
        )
      ))}
      {guides.distributionGuides.flatMap((guide, guideIndex) => (
        guide.gaps.map((gap, gapIndex) => {
          const keyBase = `${guide.draggedId}-${guide.axis}-${guideIndex}-${gapIndex}`
          if (guide.axis === 'horizontal') {
            const y = toOverlayY(gap.cross)
            const xStart = toScreenX(gap.start) + distributionCapInset
            const xEnd = toScreenX(gap.end) - distributionCapInset
            return (
              <g key={keyBase} stroke={distributionColor} strokeWidth={1.5} vectorEffect="non-scaling-stroke">
                <line x1={xStart} y1={y} x2={xEnd} y2={y} />
                <line x1={xStart} y1={y - distributionCapHalf} x2={xStart} y2={y + distributionCapHalf} />
                <line x1={xEnd} y1={y - distributionCapHalf} x2={xEnd} y2={y + distributionCapHalf} />
              </g>
            )
          }
          const x = toScreenX(gap.cross)
          const yStart = toOverlayY(gap.start) + distributionCapInset
          const yEnd = toOverlayY(gap.end) - distributionCapInset
          return (
            <g key={keyBase} stroke={distributionColor} strokeWidth={1.5} vectorEffect="non-scaling-stroke">
              <line x1={x} y1={yStart} x2={x} y2={yEnd} />
              <line x1={x - distributionCapHalf} y1={yStart} x2={x + distributionCapHalf} y2={yStart} />
              <line x1={x - distributionCapHalf} y1={yEnd} x2={x + distributionCapHalf} y2={yEnd} />
            </g>
          )
        })
      ))}
    </svg>
  )
}

function StackedCanvasItems({
  layoutData,
  fileJsonModeMap,
  hoveredEntityId,
  isDark,
  selectedEdgeIds,
  selectedEntityIdSet,
  editingEntityId,
}: {
  layoutData: LayoutUpdateData
  fileJsonModeMap: FileJsonModeMap
  hoveredEntityId: string | null
  isDark: boolean
  selectedEdgeIds: ReadonlySet<string>
  selectedEntityIdSet: Set<string>
  editingEntityId: string | null
}) {
  if (layoutData.viewMode !== 'canvas') return null

  const entitiesById = new Map(layoutData.entities.map((entity) => [entity.id, entity]))
  const edgesById = new Map(layoutData.edges.map((edge) => [edge.id, edge]))

  function renderEdge(edge: WorkspaceEdge) {
    return (
      <EdgeLayer
        key={`edge-${edge.id}`}
        edges={[edge]}
        entities={layoutData.entities}
        hoveredEntityId={hoveredEntityId}
        isDark={isDark}
        interaction={layoutData.interaction}
        selectedEdgeIds={selectedEdgeIds}
        selectedEntityIds={layoutData.selectedEntityIds}
        zoom={layoutData.zoom}
        originY={layoutData.canvasOrigin.y}
        onSelectEdge={api.selectEdge}
        renderAnchors={false}
        zIndex={undefined}
      />
    )
  }

  return (
    <>
      {layoutData.entityOrder.map((id) => {
        const edge = edgesById.get(id)
        if (edge) return renderEdge(edge)

        const entity = entitiesById.get(id)
        if (!entity) return null

        if (entity.kind === 'drawing') {
          return (
            <SavedDrawingEntities
              key={`drawing-${entity.id}`}
              entities={[entity]}
              layoutData={layoutData}
              selectedEntityIds={layoutData.selectedEntityIds}
              isDark={isDark}
            />
          )
        }

        if (entity.kind === 'shape') {
          return (
            <ShapeBodyLayer
              key={`shape-${entity.id}`}
              entities={[entity]}
              isDark={isDark}
              selectedEntityIdSet={selectedEntityIdSet}
              editingEntityId={editingEntityId}
              canvasOrigin={layoutData.canvasOrigin}
              pan={layoutData.pan}
              zoom={layoutData.zoom}
              onUpdateText={(shapeId, text) => api.updateShapeEntity(shapeId, { text })}
              onCommitEdit={api.commitEntityEdit}
            />
          )
        }

        if (entity.kind === 'text') {
          return (
            <StickyBodyLayer
              key={`text-${entity.id}`}
              entities={[entity]}
              isDark={isDark}
              selectedEntityIdSet={selectedEntityIdSet}
              editingEntityId={editingEntityId}
              canvasOrigin={layoutData.canvasOrigin}
              pan={layoutData.pan}
              zoom={layoutData.zoom}
              onUpdateText={(textId, text) => api.updateTextEntity(textId, { text })}
              onUpdateSize={(textId, width, height) =>
                api.updateTextEntity(textId, { width, height })
              }
              onCommitEdit={api.commitEntityEdit}
            />
          )
        }

        if (entity.kind === 'file') {
          return (
            <FileBodyLayer
              key={`file-${entity.id}`}
              entities={[entity]}
              isDark={isDark}
              selectedEntityIdSet={selectedEntityIdSet}
              editingEntityId={editingEntityId}
              jsonModeMap={fileJsonModeMap}
              canvasOrigin={layoutData.canvasOrigin}
              pan={layoutData.pan}
              zoom={layoutData.zoom}
              onTextEditingChange={api.setTextEditing}
            />
          )
        }

        return null
      })}
    </>
  )
}

/** Map Electron's `cursor-changed` type strings onto CSS cursor values.
 *  Electron uses Blink-era names where `pointer` is the arrow and `hand` is
 *  the link hand — the opposite of CSS. Most other types match CSS 1:1;
 *  panning variants and unknown/custom types collapse to a sensible default. */
function electronCursorToCss(type: string | null): string {
  if (!type || type === 'custom' || type === 'null') return ''
  if (type === 'pointer') return 'default'
  if (type === 'hand') return 'pointer'
  if (type === 'iBeam') return 'text'
  if (type.endsWith('-panning')) return 'all-scroll'
  return type
}

/**
 * Same-kind multi-select detector (ADR 0008 §4). Returns the array of
 * entities iff every selected id resolves to the requested kind; otherwise
 * empty. The caller mounts the popup when length >= 1.
 */
function sameKindSelectedEntities<K extends CanvasSceneEntity['kind']>(
  layout: LayoutUpdateData,
  kind: K,
): Extract<CanvasSceneEntity, { kind: K }>[] {
  const ids = layout.selectedEntityIds
  if (ids.length === 0) return []
  const result: Extract<CanvasSceneEntity, { kind: K }>[] = []
  for (const id of ids) {
    const entity = layout.entities.find((e) => e.id === id)
    if (!entity || entity.kind !== kind) return []
    result.push(entity as Extract<CanvasSceneEntity, { kind: K }>)
  }
  return result
}

function overlayRectFromScreenRect(
  rect: { left: number; top: number; width: number; height: number },
  layout: LayoutUpdateData,
) {
  return {
    ...rect,
    top: rect.top - layout.canvasOrigin.y,
  }
}

export default function App({
  initialLayoutData,
  initialTheme,
}: {
  initialLayoutData: LayoutUpdateData
  initialTheme: ThemeData
}) {
  const layoutRef = useRef<LayoutUpdateData>(initialLayoutData)
  const commentInputRef = useRef<HTMLTextAreaElement>(null)
  const threadInputRef = useRef<HTMLTextAreaElement>(null)
  const activeStrokeRef = useRef<{ pointerId: number; strokeId: string } | null>(null)
  const [layoutData, setLayoutData] = useState<LayoutUpdateData>(initialLayoutData)
  const [fixProgress, setFixProgress] = useState<LayoutUpdateData['fixProgress']>(
    initialLayoutData.fixProgress,
  )
  const [selectionOverlay, setSelectionOverlay] = useState<SelectionOverlayPayload | null>(null)
  const [canvasGuides, setCanvasGuides] = useState<CanvasGuidesPayload>({
    alignmentGuides: [],
    distributionGuides: [],
  })
  const [captureMode, setCaptureMode] = useState(false)
  const [fileJsonModeMap, setFileJsonModeMap] = useState<FileJsonModeMap>(() => new Map())
  const setFileJsonMode = useCallback((entityId: string, jsonMode: boolean) => {
    setFileJsonModeMap((prev) => {
      const next = new Map(prev)
      if (jsonMode) next.set(entityId, true)
      else next.delete(entityId)
      return next
    })
  }, [])
  useEffect(() => api.onCaptureMode(setCaptureMode), [])

  useEffect(() => api.onSelectionOverlayChanged(setSelectionOverlay), [])
  useEffect(() => api.onCanvasGuides(setCanvasGuides), [])

  // Marquee preview ids — outline layer highlights entities that the in-flight
  // marquee currently overlaps. canvas-bg used to derive this; aboveView owns
  // the marquee gesture, so we derive locally from `selectionOverlay`.
  //
  // Selection-popup mounts on single OR same-kind multi-select (ADR 0008 §4).
  // Each `selectedXxxEntities` is the non-empty array of selected entities iff
  // every selected id resolves to that kind; otherwise empty.
  const selectedTextEntities = useMemo<CanvasSceneTextEntity[]>(() => {
    return sameKindSelectedEntities(layoutData, 'text')
  }, [layoutData.selectedEntityIds, layoutData.entities])
  const selectedGroupEntity = useMemo(() => {
    if (!layoutData.selectedGroupId) return null
    return (layoutData.groups ?? []).find((g) => g.id === layoutData.selectedGroupId) ?? null
  }, [layoutData.groups, layoutData.selectedGroupId])
  const selectedShapeEntities = useMemo<CanvasSceneShapeEntity[]>(() => {
    return sameKindSelectedEntities(layoutData, 'shape')
  }, [layoutData.selectedEntityIds, layoutData.entities])
  const selectedDrawingEntities = useMemo<CanvasSceneDrawingEntity[]>(() => {
    return sameKindSelectedEntities(layoutData, 'drawing')
  }, [layoutData.selectedEntityIds, layoutData.entities])
  const selectedPageEntities = useMemo<CanvasScenePageEntity[]>(() => {
    return sameKindSelectedEntities(layoutData, 'page')
  }, [layoutData.selectedEntityIds, layoutData.entities])
  const selectedFileEntities = useMemo<CanvasSceneFileEntity[]>(() => {
    return sameKindSelectedEntities(layoutData, 'file')
  }, [layoutData.selectedEntityIds, layoutData.entities])
  const selectedEntityIdSet = useMemo(
    () => new Set(layoutData.selectedEntityIds),
    [layoutData.selectedEntityIds],
  )
  const interactionIdle = layoutData.interaction.kind === 'idle'

  // Single source of truth for "is anything currently in inline-edit mode?"
  // Derived from the broadcast interaction state — no separate ping channel.
  const editingEntityId =
    layoutData.interaction.kind === 'editing-entity'
      ? layoutData.interaction.entityId
      : null
  const textPopupReady =
    interactionIdle ||
    Boolean(
      editingEntityId &&
        selectedTextEntities.some((entity) => entity.id === editingEntityId),
    )

  const marqueePreviewIds = useMemo(() => {
    if (
      !selectionOverlay ||
      selectionOverlay.variant !== 'default' ||
      !selectionOverlay.entityIds?.length
    ) {
      return null
    }
    return new Set(selectionOverlay.entityIds)
  }, [selectionOverlay])

  const isDark = useTheme(initialTheme, api.onThemeChanged)
  useReportTextEditing(api.setTextEditing)
  useCanvasClipboard({ api, layoutRef })

  useEffect(() => {
    const cleanup = api.onLayoutUpdate((data) => {
      layoutRef.current = data
      setLayoutData(data)
      setFixProgress(data.fixProgress)
    })
    return cleanup
  }, [])

  useEffect(() => api.onFixProgressUpdate(setFixProgress), [])

  const {
    clearDraft,
    commentText,
    drawingSession,
    drawingStrokeActive,
    elementNameDraft,
    pendingAnnotation,
    pendingRegionRect,
    resizeCommentInput,
    setCommentText,
    setDrawingSession,
    setDrawingStrokeActive,
    setElementNameDraft,
    setPendingAnnotation,
    submitPendingAnnotation,
    submitRegionAnnotation,
  } = useAnnotationDraftState({
    api,
    layoutData,
    layoutRef,
    commentInputRef,
    activeStrokeRef,
  })
  const draftStateRef = useRef({ pendingAnnotation, pendingRegionRect, commentText, clearDraft })
  useEffect(() => {
    draftStateRef.current = { pendingAnnotation, pendingRegionRect, commentText, clearDraft }
  }, [pendingAnnotation, pendingRegionRect, commentText, clearDraft])
  const {
    closeThread,
    openThread,
    openThreadById,
    openThreadId,
    openThreadMenu,
    replyText,
    setOpenThreadMenu,
    setReplyText,
    submitThreadReply,
  } = useAnnotationThreadState({
    api,
    layoutData,
    threadInputRef,
  })

  // ADR 0006 — element-anchored popovers re-query their bbox via the page on
  // every scroll/resize so they don't freeze at their creation rect. Collect
  // the active subscriptions (open thread + pending element composer), hand
  // them to the live-bbox hook, then pass the resulting lookup down to the
  // popover positioners below.
  const liveBboxSubscriptions = useMemo(() => {
    const subs: Array<{ pageId: string; annotationId: string; selector: string }> = []
    if (
      pendingAnnotation &&
      pendingAnnotation.request.anchor.type === 'element'
    ) {
      const anchor = pendingAnnotation.request.anchor
      subs.push({
        pageId: anchor.pageId,
        annotationId: pendingAnnotation.draftId,
        selector: anchor.selector,
      })
    }
    if (openThread && openThread.anchor.type === 'element') {
      subs.push({
        pageId: openThread.anchor.pageId,
        annotationId: openThread.id,
        selector: openThread.anchor.selector,
      })
    }
    return subs
  }, [openThread, pendingAnnotation])

  const liveBboxes = useLiveAnnotationBboxes({ api, subscriptions: liveBboxSubscriptions })

  const threadPosition = useMemo(
    () => annotationThreadPosition(openThread, layoutData, liveBboxes),
    [layoutData, liveBboxes, openThread],
  )
  const pendingComposerPosition = useMemo(
    () => (pendingAnnotation ? pendingElementComposerPosition(pendingAnnotation, layoutData, liveBboxes) : null),
    [layoutData, liveBboxes, pendingAnnotation],
  )
  const drawInteractionEnabled = layoutData.activeTool.kind === 'draw' && !openThreadId
  const selectedEdgeIds = useMemo(() => {
    const ids = new Set<string>()
    for (const target of layoutData.selection) {
      if (target.kind === 'edge') ids.add(target.id)
    }
    return ids
  }, [layoutData.selection])
  const hoveredEntityId = layoutData.hover?.id ?? null
  const overlayInteractive = Boolean(
    pendingAnnotation ||
      pendingRegionRect ||
      openThreadId ||
      drawingSession ||
      layoutData.activeTool.kind === 'draw',
  )
  // Gate authority is main (Phase 5d-v2 D6): shouldGateBeOpen() derives
  // bounds from interaction, toolMode, modifiers, presence, marquee,
  // floating menu, and saved drawings. Main can't see renderer-local
  // state — pending composers, open thread popovers, in-flight
  // drawings — so we sync exactly those through setCommentOverlayActive.
  useEffect(() => {
    api.setCommentOverlayActive(overlayInteractive)
    return () => {
      api.setCommentOverlayActive(false)
    }
  }, [overlayInteractive])
  // Above-view is the sole owner of the placement preview ghost. The cursor
  // starts null and is set by the first pointermove (handled below); we don't
  // seed from main, because polling the OS cursor at layout time risks
  // capturing toolbar coordinates and re-snapping the ghost on every layout
  // broadcast.
  const pendingPlacement = layoutData.pendingPlacement
  const [placementCursor, setPlacementCursor] = useState<{
    clientX: number
    clientY: number
  } | null>(null)
  useEffect(() => {
    if (!pendingPlacement) setPlacementCursor(null)
  }, [pendingPlacement])
  const placementPreview = useMemo(
    () => buildPendingPlacementPreview(layoutData, placementCursor),
    [layoutData, placementCursor],
  )

  const {
    consumeSuppressedAnnotationClick,
    handleOverlayPointerCancel,
    handleOverlayPointerDown,
    handleOverlayPointerMove,
    handleOverlayPointerUp,
    startAnnotationDrag,
  } = useAnnotationDrawingGestures({
    api,
    clearDraft,
    closeThread,
    drawInteractionEnabled,
    layoutData,
    layoutRef,
    pendingAnnotation,
    activeStrokeRef,
    setDrawingSession,
    setDrawingStrokeActive,
    setPendingAnnotation,
  })

  useEffect(() => {
    api.setAnnotationState(Boolean(openThreadId), Boolean(pendingAnnotation || pendingRegionRect || drawingSession))
  }, [openThreadId, pendingAnnotation, pendingRegionRect, drawingSession])

  useRendererBindingHandlers(buildAboveViewHandlers(closeThread, clearDraft))

  // ADR 0006 page-paints contract: while the comment tool is active,
  // broadcast pointer-state to main so each page can paint a hover preview
  // (single element under the pointer; outlines for elements intersecting
  // the marquee while a region drag is in flight). We keep the broadcast
  // active during the pending region composer too so the contained-element
  // outlines stay visible while the user types — only suppress for the
  // single-target (element/canvas-point) composer where there's nothing to
  // preview.
  const commentPreviewActive =
    layoutData.activeTool.kind === 'comment' && !pendingAnnotation
  // Translate the pending region (in canvas coords) into window coords so
  // the hook can hold it across the composer. The hook prefers the
  // in-flight drag rect when both are set.
  const heldRegionRect = useMemo(() => {
    if (!pendingRegionRect) return null
    const screen = canvasRectToScreenRect(layoutData, pendingRegionRect)
    return {
      x: screen.left,
      y: screen.top,
      width: screen.width,
      height: screen.height,
    }
  }, [layoutData, pendingRegionRect])
  const commentPreview = useCommentToolPointerBroadcast({
    api,
    layoutRef,
    active: commentPreviewActive,
    heldRegionRect,
  })

  const onDragMove = useCallback(
    (startX: number, startY: number, endX: number, endY: number) => {
      const layout = layoutRef.current
      const rect = normalizeRect(startX, startY, endX, endY)
      // Annotation overlay sits at canvasOrigin.y, but the interaction overlay
      // (where the selection box renders) sits at TOOLBAR_HEIGHT. Offset the
      // rect so it aligns with the mouse in both canvas and browser modes.
      api.setSelectionOverlayRect({
        rect: {
          ...rect,
          top: rect.top + (layout.canvasOrigin.y - TOOLBAR_HEIGHT),
        },
        variant: 'region-select',
      })
      // Forward the marquee rect to the per-page hover preview so each page
      // can outline the elements its bbox intersects. Use window coords
      // (matching the pointer broadcast); main intersects with each page's
      // screen bounds and converts to page-local before forwarding.
      commentPreview.setRegionRect({
        x: rect.left,
        y: rect.top + layout.canvasOrigin.y,
        width: rect.width,
        height: rect.height,
      })
    },
    [api, commentPreview, layoutRef],
  )

  const onDragEnd = useCallback(
    (startX: number, startY: number, endX: number, endY: number) => {
      const layout = layoutRef.current
      const rect = normalizeRect(startX, startY, endX, endY)
      api.setSelectionOverlayRect(null)
      commentPreview.setRegionRect(null)
      if (rect.width < 4 || rect.height < 4) return
      // Overlay clientY is relative to the overlay top (canvasOrigin.y),
      // but clientX is already window-relative (overlay starts at x=0).
      const windowRect = {
        ...rect,
        top: rect.top + layout.canvasOrigin.y,
      }
      api.commitRegionSelect(screenRectToCanvasRect(windowRect, layout))
    },
    [api, layoutRef],
  )

  const hitTestHoverTarget = useCallback(
    (clientX: number, clientY: number) => {
      const layout = layoutRef.current
      const windowY = clientY + layout.canvasOrigin.y
      for (let i = layout.entities.length - 1; i >= 0; i--) {
        const entity = layout.entities[i]
        if (entity.kind === 'group' || entity.kind === 'drawing') continue
        if (
          clientX >= entity.screenX &&
          clientX <= entity.screenX + entity.screenWidth &&
          windowY >= entity.screenY &&
          windowY <= entity.screenY + entity.screenHeight
        ) {
          return entity.id
        }
      }
      return null
    },
    [layoutRef],
  )

  // One window pointermove handler drives both placement-preview cursor and
  // hover forwarding. When above-view intercepts events (gate open), canvas-bg
  // never sees mouseenter/leave, so we dedupe and forward via api.hoverPage.
  const lastHoverIdRef = useRef<string | null>(null)
  const hoverForwardingEnabled =
    layoutData.activeTool.kind !== 'draw' && layoutData.activeTool.kind !== 'comment'
  useEffect(() => {
    const clearHover = () => {
      setPlacementCursor(null)
      if (lastHoverIdRef.current === null) return
      lastHoverIdRef.current = null
      api.hoverPage(null)
    }
    if (!pendingPlacement && !hoverForwardingEnabled) {
      clearHover()
      return
    }
    const handleMove = (event: PointerEvent) => {
      if (isOverlayUiTarget(event.target)) {
        clearHover()
        return
      }
      if (pendingPlacement) {
        setPlacementCursor({
          clientX: event.clientX,
          clientY: event.clientY + layoutRef.current.canvasOrigin.y,
        })
      }
      if (hoverForwardingEnabled) {
        const nextId = hitTestHoverTarget(event.clientX, event.clientY)
        if (nextId !== lastHoverIdRef.current) {
          lastHoverIdRef.current = nextId
          api.hoverPage(nextId)
        }
      }
    }
    // The top toolbar is a sibling WebContentsView, so when the cursor moves
    // up into it the above-view stops receiving pointer events without
    // pointerleave firing. mouseleave on documentElement is the reliable
    // "cursor left this webcontents" signal in Electron's multi-view layout.
    const docEl = document.documentElement
    window.addEventListener('pointermove', handleMove)
    // eslint-disable-next-line local/no-mouse-events
    docEl.addEventListener('mouseleave', clearHover)
    window.addEventListener('blur', clearHover)
    return () => {
      window.removeEventListener('pointermove', handleMove)
      // eslint-disable-next-line local/no-mouse-events
      docEl.removeEventListener('mouseleave', clearHover)
      window.removeEventListener('blur', clearHover)
      clearHover()
    }
  }, [api, hitTestHoverTarget, hoverForwardingEnabled, layoutRef, pendingPlacement])

  const routerOwnsCanvasPointers =
    !overlayInteractive &&
    !pendingPlacement &&
    !isAnnotationTool(layoutData.activeTool)
  const toolGestureOwnsCanvasPointers =
    !overlayInteractive &&
    (Boolean(pendingPlacement) || layoutData.activeTool.kind === 'comment')

  // Comment-tool gesture + placement-tool gesture share this overlay handler:
  // both capture pointerdown/move/up while the gate routes events to aboveView.
  //
  // Comment tool (ADR 0006): click below threshold → resolve element under
  // cursor via `inspectAtPoint`; element hit → element anchor; nothing →
  // canvas-point anchor. Drag past threshold → marquee → region anchor on
  // pointerup. Threshold matches the rest of the canvas pointer router.
  const COMMENT_DRAG_THRESHOLD = 4
  // The comment tool needs to keep capturing pointerdowns while a pending
  // annotation or region rect is open so the user can retarget by clicking a
  // different element. `isOverlayUiTarget` below still filters out clicks on
  // the composer / popups so typing isn't interrupted.
  const commentToolBlocked = Boolean(
    openThreadId || drawingSession || layoutData.activeTool.kind === 'draw',
  )
  const skipPointerCapture =
    layoutData.activeTool.kind === 'comment' ? commentToolBlocked : overlayInteractive
  useEffect(() => {
    if (skipPointerCapture) return
    if (!pendingPlacement && layoutData.activeTool.kind !== 'comment') return

    const onPointerDown = (event: PointerEvent) => {
      if (isOverlayUiTarget(event.target)) return
      if (event.button !== 0) return
      const layout = layoutRef.current
      if (layout.viewMode !== 'canvas') return
      if (!layout.pendingPlacement && layout.activeTool.kind !== 'comment') return

      event.preventDefault()
      event.stopPropagation()

      const pointerId = event.pointerId
      const target = event.target instanceof Element ? event.target : null
      try {
        target?.setPointerCapture(pointerId)
      } catch {
        /* ignore */
      }

      const startX = event.clientX
      const startY = event.clientY
      const startWindowY = startY + layout.canvasOrigin.y
      const startCanvas = screenPointToCanvasPoint(startX, startWindowY, layout)
      const placementAtStart = layout.pendingPlacement
      let crossedThreshold = false

      const cleanup = () => {
        try {
          if (target?.hasPointerCapture(pointerId)) target.releasePointerCapture(pointerId)
        } catch {
          /* ignore */
        }
        window.removeEventListener('pointermove', onMove)
        window.removeEventListener('pointerup', onUp)
        window.removeEventListener('pointercancel', onCancel)
        window.removeEventListener('blur', onCancel)
      }

      const updateShapePreview = (ev: PointerEvent) => {
        const current = layoutRef.current
        const endCanvas = screenPointToCanvasPoint(
          ev.clientX,
          ev.clientY + current.canvasOrigin.y,
          current,
        )
        const square = squareConstrainedRect(
          startCanvas.x,
          startCanvas.y,
          endCanvas.x,
          endCanvas.y,
          ev.shiftKey,
        )
        const minCanvasX = snapToGrid(square.left)
        const minCanvasY = snapToGrid(square.top)
        const snappedW = snapToGrid(square.width)
        const snappedH = snapToGrid(square.height)
        const screenRect = {
          left: canvasToScreenX(current, minCanvasX),
          top: canvasToScreenY(current, minCanvasY),
          width: snappedW * current.zoom,
          height: snappedH * current.zoom,
        }
        api.setSelectionOverlayRect({
          rect: overlayRectFromScreenRect(screenRect, current),
          variant: 'place-shape',
          shapeKind: current.pendingPlacement?.shapeKind ?? 'rectangle',
        })
      }

      const onMove = (ev: PointerEvent) => {
        if (ev.pointerId !== pointerId) return
        const current = layoutRef.current
        if (placementAtStart?.entityKind === 'shape') {
          updateShapePreview(ev)
          return
        }
        if (!placementAtStart && current.activeTool.kind === 'comment') {
          if (!crossedThreshold) {
            const dx = ev.clientX - startX
            const dy = ev.clientY - startY
            if (Math.abs(dx) < COMMENT_DRAG_THRESHOLD && Math.abs(dy) < COMMENT_DRAG_THRESHOLD) {
              return
            }
            crossedThreshold = true
          }
          onDragMove(startX, startY, ev.clientX, ev.clientY)
        }
      }

      const onUp = (ev: PointerEvent) => {
        if (ev.pointerId !== pointerId) return
        cleanup()
        const current = layoutRef.current
        if (placementAtStart) {
          if (placementAtStart.entityKind === 'shape') {
            api.setSelectionOverlayRect(null)
            const endCanvas = screenPointToCanvasPoint(
              ev.clientX,
              ev.clientY + current.canvasOrigin.y,
              current,
            )
            const square = squareConstrainedRect(
              startCanvas.x,
              startCanvas.y,
              endCanvas.x,
              endCanvas.y,
              ev.shiftKey,
            )
            if (square.width >= MIN_SHAPE_DRAG_SIZE && square.height >= MIN_SHAPE_DRAG_SIZE) {
              api.placePendingShape(snapToGrid(square.left), snapToGrid(square.top), {
                x: snapToGrid(square.left),
                y: snapToGrid(square.top),
                width: snapToGrid(square.width),
                height: snapToGrid(square.height),
              })
            } else {
              api.placePendingShape(snapToGrid(startCanvas.x), snapToGrid(startCanvas.y), null)
            }
            return
          }
          api.placePendingEntity(snapToGrid(startCanvas.x), snapToGrid(startCanvas.y))
          return
        }
        if (current.activeTool.kind === 'comment') {
          if (crossedThreshold) {
            // Drag past threshold → region anchor.
            onDragEnd(startX, startY, ev.clientX, ev.clientY)
            return
          }
          // Click below threshold → element anchor if a page DOM element sits
          // under the cursor (resolved via `inspectAtPoint`), else canvas-point.
          api.setSelectionOverlayRect(null)
          const draft = draftStateRef.current
          const hasEmptyDraft =
            Boolean(draft.pendingAnnotation || draft.pendingRegionRect) &&
            !draft.commentText.trim()
          if (hasEmptyDraft) {
            // Empty composer open → click-away dismisses it without creating
            // a new draft; comment mode stays active.
            draft.clearDraft()
            return
          }
          api.commitCommentClickAt(ev.clientX, ev.clientY + current.canvasOrigin.y)
        }
      }

      const onCancel = () => {
        cleanup()
        api.setSelectionOverlayRect(null)
      }

      window.addEventListener('pointermove', onMove)
      window.addEventListener('pointerup', onUp)
      window.addEventListener('pointercancel', onCancel)
      window.addEventListener('blur', onCancel)
    }

    window.addEventListener('pointerdown', onPointerDown, { capture: true })
    return () => {
      window.removeEventListener('pointerdown', onPointerDown, {
        capture: true,
      } as EventListenerOptions)
    }
  }, [api, layoutData.activeTool.kind, layoutRef, onDragEnd, onDragMove, pendingPlacement, skipPointerCapture])

  const viewportWheelAndPanApi = useMemo(
    () => ({
      canvasZoom: api.canvasZoom,
      canvasPan: api.canvasPan,
    }),
    [],
  )
  // Pre-route wheel events that hit the single-selected page's body into
  // that page's page. Cmd/Ctrl+wheel is already classified as 'zoom' by
  // useViewportWheelAndMiddlePan and stays on the canvas. Wheel during a
  // drag/marquee/edge gesture also stays with the canvas — forwarding it
  // would scroll the page underneath an in-flight gesture.
  const routeWheel = useCallback(
    (event: WheelEvent): boolean => {
      const layout = layoutRef.current
      if (layout.viewMode !== 'canvas') return false
      if (layout.interaction.kind !== 'idle') return false
      const selected = layout.selectedEntityIds
      if (selected.length !== 1) return false
      const pageId = selected[0]
      const page = layout.entities.find(
        (entity): entity is CanvasSceneEntity & { kind: 'page' } =>
          entity.kind === 'page' && entity.id === pageId,
      )
      if (!page) return false
      const windowY = event.clientY + layout.canvasOrigin.y
      const x0 = page.contentScreenX ?? page.screenX
      const y0 = page.contentScreenY ?? page.screenY
      const x1 = x0 + (page.contentScreenWidth ?? page.screenWidth)
      const y1 = y0 + (page.contentScreenHeight ?? page.screenHeight)
      if (event.clientX < x0 || event.clientX > x1) return false
      if (windowY < y0 || windowY > y1) return false
      api.forwardWheelToPage(pageId, {
        windowX: event.clientX,
        windowY,
        deltaX: event.deltaX,
        deltaY: event.deltaY,
        // DOM_DELTA_PIXEL on macOS trackpads → precise; line/page mode → ticks.
        hasPreciseScrollingDeltas: event.deltaMode === 0,
        // Cmd/Ctrl+wheel is intercepted by classifyViewportWheel as 'zoom'
        // and never reaches us, so 'pan' here always scrolls.
        canScroll: true,
        shiftKey: event.shiftKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        metaKey: event.metaKey,
      })
      return true
    },
    [layoutRef],
  )
  useViewportWheelAndMiddlePan(true, viewportWheelAndPanApi, routeWheel)

  // PoC: mirror the focused page's `cursor-changed` onto aboveView's body so
  // the OS shows the right cursor (hand on links, I-beam on text, etc.). The
  // OS picks cursor from the topmost WCV at the pointer location, which is
  // aboveView whenever the canvas-mode gate is open.
  useEffect(() => {
    return api.onPageCursorChange(({ type }) => {
      document.body.style.cursor = electronCursorToCss(type)
    })
  }, [])

  // PoC: continuous hover forwarding into the single-selected page's body so
  // cursor styling (link → hand, text → I-beam) and hover-driven UI react
  // without requiring a button-down. The router's `runForwardPointer` already
  // forwards moves while a button is held, so this listener only fires when
  // no buttons are pressed to avoid double-dispatch. When the pointer leaves
  // the focused page's body (or selection drops below one page), reset
  // body cursor so the hand/I-beam doesn't bleed into canvas chrome.
  useEffect(() => {
    let cursorIsForwarded = false
    const resetCursor = () => {
      if (!cursorIsForwarded) return
      cursorIsForwarded = false
      document.body.style.cursor = ''
    }
    const onMove = (event: PointerEvent) => {
      if (event.buttons !== 0) return
      const layout = layoutRef.current
      if (layout.viewMode !== 'canvas') return resetCursor()
      const selected = layout.selectedEntityIds
      if (selected.length !== 1) return resetCursor()
      const pageId = selected[0]
      const page = layout.entities.find(
        (entity): entity is CanvasSceneEntity & { kind: 'page' } =>
          entity.kind === 'page' && entity.id === pageId,
      )
      if (!page) return resetCursor()
      const windowY = event.clientY + layout.canvasOrigin.y
      const x0 = page.contentScreenX ?? page.screenX
      const y0 = page.contentScreenY ?? page.screenY
      const x1 = x0 + (page.contentScreenWidth ?? page.screenWidth)
      const y1 = y0 + (page.contentScreenHeight ?? page.screenHeight)
      if (event.clientX < x0 || event.clientX > x1) return resetCursor()
      if (windowY < y0 || windowY > y1) return resetCursor()
      cursorIsForwarded = true
      api.forwardPointerToPage(pageId, {
        kind: 'move',
        windowX: event.clientX,
        windowY,
        button: 'left',
        shiftKey: event.shiftKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        metaKey: event.metaKey,
      })
    }
    window.addEventListener('pointermove', onMove)
    return () => {
      window.removeEventListener('pointermove', onMove)
      resetCursor()
    }
  }, [layoutRef])

  // ADR 0001 — canvas pointer router. Single window-level pointerdown
  // listener that runs the shared hit-test, classifies the action via the
  // priority table, and dispatches every gesture (focus, drag, resize,
  // edge-drag, marquee, pan) through the existing IPC surface. The
  // `EdgeDragLayer` below renders the rubber-band line driven by the same
  // controller state.
  const spaceHeldRef = useRef(false)
  const optionHeldRef = useRef(false)
  const handToolActiveRef = useRef(layoutData.activeTool.kind === 'hand')
  handToolActiveRef.current = layoutData.activeTool.kind === 'hand'
  useEffect(() => {
    const onKey = (event: KeyboardEvent, down: boolean) => {
      if (event.code === 'Space') spaceHeldRef.current = down
      if (event.key === 'Alt' || event.code === 'AltLeft' || event.code === 'AltRight') {
        optionHeldRef.current = down
      }
    }
    const onDown = (e: KeyboardEvent) => onKey(e, true)
    const onUp = (e: KeyboardEvent) => onKey(e, false)
    const onBlur = () => {
      spaceHeldRef.current = false
      optionHeldRef.current = false
    }
    window.addEventListener('keydown', onDown)
    window.addEventListener('keyup', onUp)
    window.addEventListener('blur', onBlur)
    return () => {
      window.removeEventListener('keydown', onDown)
      window.removeEventListener('keyup', onUp)
      window.removeEventListener('blur', onBlur)
    }
  }, [])
  const [edgeDragState, setEdgeDragState] = useState<EdgeDragState>(EDGE_DRAG_IDLE)
  const [dragCopyPreview, setDragCopyPreview] = useState<DragCopyPreviewBox[]>([])
  useCanvasPointerRouter({
    api,
    layoutRef,
    enabled: routerOwnsCanvasPointers,
    consume: FULL_ROUTER_CONSUME,
    spaceHeldRef,
    handToolActiveRef,
    optionHeldRef,
    setDragCopyPreview,
    setEdgeDragState,
  })

  useEffect(() => {
    if (!pendingAnnotation) return
    closeThread()
  }, [closeThread, pendingAnnotation])

  useEffect(() => {
    if (!openThreadId) return
    activeStrokeRef.current = null
    clearDraft()
  }, [clearDraft, openThreadId])

  useEffect(() => {
    if (!drawInteractionEnabled) return
    // Force the pen cursor across every element while in draw mode — some
    // children (drawing hit-paths, thread chrome, region annotations) set
    // their own cursor and would otherwise win on hover.
    const style = document.createElement('style')
    style.textContent = `html, body, body * { cursor: ${DRAW_CURSOR} !important; }`
    document.head.appendChild(style)
    return () => {
      style.remove()
    }
  }, [drawInteractionEnabled])

  const handToolActive = layoutData.activeTool.kind === 'hand'
  useEffect(() => {
    if (!handToolActive) return
    const style = document.createElement('style')
    style.textContent = `html, body, body * { cursor: grab !important; }
html:active, body:active, body *:active { cursor: grabbing !important; }`
    document.head.appendChild(style)
    return () => {
      style.remove()
    }
  }, [handToolActive])

  return (
    <div
      className={`relative h-screen w-screen overflow-hidden bg-transparent ${
        overlayInteractive || routerOwnsCanvasPointers || toolGestureOwnsCanvasPointers
          ? 'pointer-events-auto'
          : 'pointer-events-none'
      }`}
      style={{
        cursor: drawInteractionEnabled ? DRAW_CURSOR : undefined,
      }}
      onPointerDown={handleOverlayPointerDown}
      onPointerMove={handleOverlayPointerMove}
      onPointerUp={handleOverlayPointerUp}
      onPointerCancel={handleOverlayPointerCancel}
    >
      {placementPreview && selectionOverlay?.variant !== 'place-shape' ? (
        <PlacementPreviewLayer
          isDark={isDark}
          preview={{
            ...placementPreview,
            top: placementPreview.top - layoutData.canvasOrigin.y,
          }}
        />
      ) : null}

      {selectionOverlay?.variant === 'place-shape' &&
      selectionOverlay.rect.width > 0 &&
      selectionOverlay.rect.height > 0 ? (
        <PlacementPreviewLayer
          isDark={isDark}
          preview={{
            entityKind: 'shape',
            shapeKind: selectionOverlay.shapeKind,
            left: selectionOverlay.rect.left,
            top: selectionOverlay.rect.top,
            width: selectionOverlay.rect.width,
            height: selectionOverlay.rect.height,
          }}
        />
      ) : null}

      {!captureMode ? (
        <>
          {/* ADR 0006: region anchors always render their resting visual,
              filtered only by status. Element + canvas-point anchors have no
              resting chrome — they live in the right panel. */}
          <RegionSelectAnnotations
            annotations={layoutData.annotations}
            interactive={!selectionOverlay && !pendingRegionRect && !pendingAnnotation}
            layoutData={layoutData}
            onOpenThread={openThreadById}
          />

          {drawingSession ? <DrawingLayer drawing={{ version: 1, ...drawingSession }} layout={layoutData} active isDark={isDark} /> : null}

          <PendingElementOutline
            pending={pendingAnnotation}
            layoutData={layoutData}
            liveBboxes={liveBboxes}
          />

          <PendingAnnotationComposer
            clearDraft={clearDraft}
            commentInputRef={commentInputRef}
            commentText={commentText}
            elementNameDraft={elementNameDraft}
            layoutData={layoutData}
            pendingAnnotation={pendingAnnotation}
            pendingPosition={pendingComposerPosition}
            pendingRegionRect={pendingRegionRect}
            resizeCommentInput={resizeCommentInput}
            setCommentText={setCommentText}
            setElementNameDraft={setElementNameDraft}
            submitPendingAnnotation={submitPendingAnnotation}
            submitRegionAnnotation={submitRegionAnnotation}
          />

        </>
      ) : null}

      {!captureMode ? (
        <>
          <AnnotationThreadPopover
            api={api}
            closeThread={closeThread}
            drawCursor={DRAW_CURSOR}
            drawInteractionEnabled={drawInteractionEnabled}
            openThread={openThread}
            openThreadMenu={openThreadMenu}
            progress={openThread ? fixProgress[openThread.id] : undefined}
            replyText={replyText}
            setOpenThreadMenu={setOpenThreadMenu}
            setReplyText={setReplyText}
            startAnnotationDrag={startAnnotationDrag}
            submitThreadReply={submitThreadReply}
            threadInputRef={threadInputRef}
            threadPosition={threadPosition}
          />

          <MarqueeLayer overlay={selectionOverlay} />

          {layoutData.viewMode === 'canvas' && layoutData.presenceCursors.length > 0 ? (
            <ActivePageHighlightLayer
              cursors={layoutData.presenceCursors}
              pages={layoutData.entities.filter(
                (e): e is CanvasScenePageEntity => e.kind === 'page',
              )}
              originY={layoutData.canvasOrigin.y}
            />
          ) : null}

          <StackedCanvasItems
            layoutData={layoutData}
            fileJsonModeMap={fileJsonModeMap}
            hoveredEntityId={hoveredEntityId}
            isDark={isDark}
            selectedEdgeIds={selectedEdgeIds}
            selectedEntityIdSet={selectedEntityIdSet}
            editingEntityId={editingEntityId}
          />

          {layoutData.viewMode === 'canvas' ? (
            <EdgeLayer
              edges={[]}
              entities={layoutData.entities}
              hoveredEntityId={hoveredEntityId}
              isDark={isDark}
              interaction={layoutData.interaction}
              selectedEdgeIds={selectedEdgeIds}
              selectedEntityIds={layoutData.selectedEntityIds}
              zoom={layoutData.zoom}
              originY={layoutData.canvasOrigin.y}
              onSelectEdge={api.selectEdge}
            />
          ) : null}

          {layoutData.viewMode === 'canvas' && (layoutData.groups?.length ?? 0) > 0 ? (
            <GroupBoundsLayer
              groups={layoutData.groups ?? []}
              isDark={isDark}
              zoom={layoutData.zoom}
              canvasOrigin={layoutData.canvasOrigin}
              pan={layoutData.pan}
            />
          ) : null}

          {layoutData.viewMode === 'canvas' ? (
            <PageFocusRingLayer
              pages={layoutData.entities.filter(
                (e): e is CanvasScenePageEntity => e.kind === 'page',
              )}
              fileEntities={layoutData.entities.filter(
                (e): e is CanvasSceneFileEntity => e.kind === 'file',
              )}
              focusedPageId={layoutData.keyboardTargetPageId}
              originY={layoutData.canvasOrigin.y}
            />
          ) : null}

          {layoutData.viewMode === 'canvas' ? (
            <SelectionOutlineLayer
              layoutData={layoutData}
              isDark={isDark}
              marqueePreviewIds={marqueePreviewIds}
            />
          ) : null}

          <EdgeDragLayer state={edgeDragState} layoutData={layoutData} isDark={isDark} />
          <DragCopyPreviewLayer previews={dragCopyPreview} isDark={isDark} />
          <GuideOverlayLayer guides={canvasGuides} layoutData={layoutData} isDark={isDark} />

          <PageChromeOverlay
            api={api}
            layoutData={layoutData}
            isDark={isDark}
            optionHeldRef={optionHeldRef}
            setDragCopyPreview={setDragCopyPreview}
          />
          <FileChromeOverlay
            api={api}
            layoutData={layoutData}
            isDark={isDark}
            optionHeldRef={optionHeldRef}
            setDragCopyPreview={setDragCopyPreview}
          />
          <GroupRenameOverlay
            api={api}
            layoutData={layoutData}
            isDark={isDark}
            editingEntityId={editingEntityId}
            optionHeldRef={optionHeldRef}
            setDragCopyPreview={setDragCopyPreview}
          />

          {layoutData.viewMode === 'canvas' ? (
            <>
              {/* Tool-mode popups (ADR 0008 §2 mutex: tool wins when active). */}
              {layoutData.activeTool.kind === 'add-text' ? (
                <TextToolPopup
                  api={api}
                  isDark={isDark}
                  layout={layoutData}
                  style="plain"
                />
              ) : null}
              {layoutData.activeTool.kind === 'add-sticky' ? (
                <TextToolPopup
                  api={api}
                  isDark={isDark}
                  layout={layoutData}
                  style="sticky"
                />
              ) : null}
              {layoutData.activeTool.kind === 'add-shape' ? (
                <ShapeToolPopup api={api} isDark={isDark} layout={layoutData} />
              ) : null}
              {layoutData.activeTool.kind === 'draw' ? (
                <DrawToolPopup api={api} isDark={isDark} layout={layoutData} />
              ) : null}

              {/* Selection-mode popups — suppressed while any tool with its own
                  popup is active (ADR 0008 §2). */}
              {!toolHasPopup(layoutData.activeTool) ? (
                <>
                  <StickyNotePopover
                    api={api}
                    isDark={isDark}
                    layout={layoutData}
                    selectedTextEntities={selectedTextEntities}
                    popupReady={textPopupReady}
                  />
                  <GroupPopup
                    api={api}
                    isDark={isDark}
                    layout={layoutData}
                    selectedGroup={selectedGroupEntity}
                    interactionIdle={interactionIdle}
                  />
                  <ShapePopup
                    api={api}
                    isDark={isDark}
                    layout={layoutData}
                    selectedShapes={selectedShapeEntities}
                    interactionIdle={interactionIdle}
                  />
                  <DrawingPopup
                    api={api}
                    isDark={isDark}
                    layout={layoutData}
                    selectedDrawings={selectedDrawingEntities}
                    interactionIdle={interactionIdle}
                  />
                  <PagePopup
                    api={api}
                    isDark={isDark}
                    layout={layoutData}
                    selectedPages={selectedPageEntities}
                    interactionIdle={interactionIdle}
                  />
                  <FilePopup
                    api={api}
                    isDark={isDark}
                    layout={layoutData}
                    selectedFiles={selectedFileEntities}
                    interactionIdle={interactionIdle}
                    fileJsonModeMap={fileJsonModeMap}
                    setFileJsonMode={setFileJsonMode}
                  />
                </>
              ) : null}
            </>
          ) : null}
        </>
      ) : null}
    </div>
  )
}
