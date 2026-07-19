import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { CanvasSceneEntity, CanvasSceneFileEntity, CanvasScenePageEntity, LayoutUpdateData, SelectionOverlayPayload, ThemeData, WorkspaceEdge } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { CanvasGuidesPayload } from '../../shared/canvas-guides'
import {
  canScrollWheelTarget,
  clientYToWindowY,
  normalizeRect,
  screenRectToCanvasRect,
} from '../../shared/gesture-utils'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import { toolHasPopup } from '../../shared/tool'
import {
  annotationOverlayActive,
  canvasPointerOwner,
} from '../../shared/canvas-pointer-owner'
import { isUnresolved } from '../../shared/annotation-utils'
import { DRAW_CURSOR, selectionColor } from '../canvas-bg/canvasBgConstants'
import { PlacementPreviewLayer } from '../canvas-bg/CanvasGridSurface'
import { buildPendingPlacementPreview } from '../canvas-bg/canvasBgSelectors'
import { DrawingLayer, SavedDrawingEntities } from './DrawingsLayer'
import { FileBodyLayer } from './FileBodyLayer'
import { focusContext } from '../../shared/focus-context'
import { PageFocusRingLayer } from './PageFocusRingLayer'
import { GroupBoundsLayer } from './GroupBoundsLayer'
import { SelectionOutlineLayer, type SelectedEntitySpan } from './SelectionOutlineLayer'
import { ShapeBodyLayer } from './ShapeBodyLayer'
import { StickyBodyLayer } from './StickyBodyLayer'
import { RegionSelectAnnotations } from './AnnotationsLayer'
import { CommentBadgesLayer } from './CommentBadgesLayer'
import {
  AnnotationThreadPopover,
  PendingAnnotationComposer,
  PendingElementOutline,
} from './CommentsLayer'
import { MarqueeLayer } from './MarqueeLayer'
import { useAnnotationDrawingGestures } from './useAnnotationDrawingGestures'
import { useAnnotationDraftState } from './useAnnotationDraftState'
import {
  useAnnotationThreadState,
  annotationThreadPosition,
} from './useAnnotationThreadState'
import { useCommentToolPointerBroadcast } from './useCommentToolPointerBroadcast'
import { useLiveAnnotationBboxes } from './useLiveAnnotationBboxes'
import { useCanvasFileDrop } from './useCanvasFileDrop'
import { canvasRectToScreenRect, pendingElementComposerPosition } from './annotationMath'
import {
  FULL_ROUTER_CONSUME,
  useCanvasPointerRouter,
  type ReorderGhostOffset,
} from './useCanvasPointerRouter'
import { usePageInputForwarding } from './usePageInputForwarding'
import { pointerOverPageContent } from '../../shared/page-hit-test'
import { EdgeDragLayer } from './EdgeDragLayer'
import { EdgeLayer } from './EdgeLayer'
import { EdgePopup } from './EdgePopup'
import { edgeForPopup } from './edgePopupSelection'
import { ReorderDotsLayer } from './ReorderDotsLayer'
import { reorderPreviewLayout } from './reorderPreview'
import { gapPreviewLayout } from './gapPreview'
import { GapHandlesLayer } from './GapHandlesLayer'
import { GroupRenameOverlay } from './GroupRenameLabel'
import {
  computeSameKindSelection,
  sameKindEntities,
  SELECTION_POPUPS,
  TOOL_POPUPS,
  type PopupContext,
} from './canvasItemPopupTable'
import { EDGE_DRAG_IDLE, type EdgeDragState } from '../../shared/edge-drag-controller'
import type { DragCopyPreviewBox } from './optionDragCopy'
import { useCanvasClipboard } from '../canvas-bg/useCanvasClipboard'
import { buildAboveViewHandlers } from './binding-handlers'
import { useReportTextEditing } from '../shared/hooks/useReportTextEditing'
import { useRendererBindingHandlers } from '../shared/hooks/useRendererBindingHandlers'
import { useScenePanOffset } from '../shared/hooks/useScenePanOffset'
import { useTheme } from '../shared/hooks/useTheme'
import { useViewportWheelAndMiddlePan } from '../shared/hooks/useViewportWheelAndMiddlePan'

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

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
          className="pointer-events-none absolute border"
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
  hoveredEntityId,
  isDark,
  selectedEdgeIds,
  selectedEntityIdSet,
  editingEntityId,
  interactiveEntityId,
  ghostEntity,
  hideContext,
}: {
  layoutData: LayoutUpdateData
  hoveredEntityId: string | null
  isDark: boolean
  selectedEdgeIds: ReadonlySet<string>
  selectedEntityIdSet: Set<string>
  editingEntityId: string | null
  /** Interactive file (HTML iframe) the user has entered, or null. Flips that
   *  iframe's pointer-events on so scroll/clicks reach its content. */
  interactiveEntityId: string | null
  /** Focus is at rest with the eye off — skip all non-page context (annotation
   *  entities, edges, file entities) entirely (binary, never dimmed). ADR 0021. */
  hideContext: boolean
  /** Reorder ghost (ADR 0015 D7, Phase D): the dragged entity, already
   *  positioned at grab-origin + cursor-delta. Its in-row slot paints as a
   *  grayscale placeholder (the drop location); the ghost itself renders last at
   *  50% opacity, floating over the settling siblings under the cursor. */
  ghostEntity?: CanvasSceneEntity | null
}) {
  const entitiesById = new Map(layoutData.entities.map((entity) => [entity.id, entity]))
  const edgesById = new Map(layoutData.edges.map((edge) => [edge.id, edge]))

  function renderEdge(edge: WorkspaceEdge) {
    if (hideContext) return null
    const layer = (
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
    return layer
  }

  function renderEntityBody(entity: CanvasSceneEntity) {
    // Eye off: hide every non-page item — annotations *and* files/images. The
    // focused page is a webview, not rendered here, so it's never affected
    // (focus is always page-targeted). ADR 0021.
    if (hideContext) return null
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
          layoutData={layoutData}
          onUpdateText={(shapeId, text) => api.updateEntity('shape', shapeId, { text })}
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
          layoutData={layoutData}
          onUpdateText={(textId, text) => api.updateEntity('text', textId, { text })}
          onUpdateSize={(textId, width, height) =>
            api.updateEntity('text', textId, { width, height })
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
          interactiveEntityId={interactiveEntityId}
          canvasOrigin={layoutData.canvasOrigin}
          pan={layoutData.pan}
          zoom={layoutData.zoom}
          onTextEditingChange={api.setTextEditing}
        />
      )
    }
    return null
  }

  return (
    <>
      {layoutData.entityOrder.map((id) => {
        const edge = edgesById.get(id)
        if (edge) return renderEdge(edge)

        const entity = entitiesById.get(id)
        if (!entity) return null
        // The dragged item's slot paints as a grayscale placeholder — the live
        // drop location, sitting at the packed destination slot (it snaps here at
        // the 50% threshold). The full-colour ghost floats over it under the
        // cursor (rendered last, below).
        if (ghostEntity && entity.id === ghostEntity.id) {
          return (
            <div
              key={`reorder-placeholder-${entity.id}`}
              className="pointer-events-none"
              style={{ filter: 'grayscale(1)', opacity: 0.45 }}
            >
              {renderEntityBody(entity)}
            </div>
          )
        }

        return renderEntityBody(entity)
      })}
      {ghostEntity ? (
        <div key="reorder-ghost" className="pointer-events-none" style={{ opacity: 0.5 }}>
          {renderEntityBody(ghostEntity)}
        </div>
      ) : null}
    </>
  )
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
  const panOffset = useScenePanOffset(api.onViewportNudge, layoutData)
  const [fixProgress, setFixProgress] = useState<LayoutUpdateData['fixProgress']>(
    initialLayoutData.fixProgress,
  )
  const [selectionOverlay, setSelectionOverlay] = useState<SelectionOverlayPayload | null>(null)
  const [canvasGuides, setCanvasGuides] = useState<CanvasGuidesPayload>({
    alignmentGuides: [],
    distributionGuides: [],
  })
  const [captureMode, setCaptureMode] = useState(false)
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
  const sameKindSelection = useMemo(
    () => computeSameKindSelection(layoutData),
    [layoutData.selectedEntityIds, layoutData.entities],
  )
  const selectedGroupEntity = useMemo(() => {
    if (!layoutData.selectedGroupId) return null
    return (layoutData.groups ?? []).find((g) => g.id === layoutData.selectedGroupId) ?? null
  }, [layoutData.groups, layoutData.selectedGroupId])
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
        sameKindEntities(sameKindSelection, 'text').some(
          (entity) => entity.id === editingEntityId,
        ),
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

  const { isDark } = useTheme(initialTheme, api.onThemeChanged)

  // Reorder ghost (ADR 0015 D7, Phase D): the canvas-space pointer delta since
  // the dragged item was grabbed, published by the gesture in
  // `useCanvasPointerRouter`. Drives the floating 50%-opacity ghost; null when
  // not reordering.
  const [reorderGhost, setReorderGhost] = useState<ReorderGhostOffset>(null)

  // During a reorder drag the *siblings* render at their previewed slots so the
  // row visibly opens a gap to receive the dragged item (ADR 0015 D7, Phase D).
  // The dragged item's slot stays reserved here but its body is skipped (drawn
  // as the ghost instead), so the reserved slot reads as the open gap. Pure
  // renderer ephemera — the broadcast layout is untouched. Falls back to the
  // broadcast layout when not reordering.
  const renderLayout = useMemo(
    () => reorderPreviewLayout(layoutData) ?? gapPreviewLayout(layoutData) ?? layoutData,
    [layoutData],
  )

  // The dragged entity floated at grab-origin + cursor-delta. Read from the
  // *broadcast* layout (its untouched resting position) — never `renderLayout`,
  // where it sits in its packed slot. Both canvas and screen coords shift so
  // whichever a body layer reads lands the ghost under the cursor.
  const reorderGhostEntity = useMemo<CanvasSceneEntity | null>(() => {
    const interaction = layoutData.interaction
    if (interaction.kind !== 'reordering-row') return null
    const moving = layoutData.entities.find((e) => e.id === interaction.movingId)
    if (!moving) return null
    const dx = reorderGhost?.dx ?? 0
    const dy = reorderGhost?.dy ?? 0
    const { zoom } = layoutData
    return {
      ...moving,
      canvasX: moving.canvasX + dx,
      canvasY: moving.canvasY + dy,
      screenX: moving.screenX + dx * zoom,
      screenY: moving.screenY + dy * zoom,
    }
  }, [layoutData, reorderGhost])

  // The drop-location placeholder sits at the dragged item's packed destination
  // slot (its position in `renderLayout`, which snaps at the 50% threshold) — so
  // the multi-select bounding box wraps it via that layout. To keep the box
  // *resizing toward the cursor* during the drag (FigJam parity), also feed the
  // lifted item's ghost rect as an extra bounding span. Never a group — reorder
  // targets entities only.
  const reorderGhostSpan = useMemo<SelectedEntitySpan | null>(() => {
    if (!reorderGhostEntity || reorderGhostEntity.kind === 'group') return null
    return reorderGhostEntity
  }, [reorderGhostEntity])

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
    const seen = new Set<string>()
    const pushSub = (sub: { pageId: string; annotationId: string; selector: string }) => {
      const key = `${sub.pageId}:${sub.annotationId}:${sub.selector}`
      if (seen.has(key)) return
      seen.add(key)
      subs.push(sub)
    }
    if (
      pendingAnnotation &&
      pendingAnnotation.request.anchor.type === 'element'
    ) {
      const anchor = pendingAnnotation.request.anchor
      pushSub({
        pageId: anchor.pageId,
        annotationId: pendingAnnotation.draftId,
        selector: anchor.selector,
      })
    }
    if (openThread && openThread.anchor.type === 'element') {
      pushSub({
        pageId: openThread.anchor.pageId,
        annotationId: openThread.id,
        selector: openThread.anchor.selector,
      })
    }
    for (const annotation of layoutData.annotations) {
      if (!isUnresolved(annotation.status) || annotation.anchor.type !== 'element') continue
      pushSub({
        pageId: annotation.anchor.pageId,
        annotationId: annotation.id,
        selector: annotation.anchor.selector,
      })
    }
    return subs
  }, [layoutData.annotations, openThread, pendingAnnotation])

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
  // The single edge must also be the entire selection. Otherwise the combined
  // selection popup owns the interaction surface.
  const selectedEdge = useMemo(() => {
    return edgeForPopup(layoutData.selection, layoutData.edges)
  }, [layoutData.selection, layoutData.edges])
  const hoveredEntityId = layoutData.hover?.id ?? null
  const focus = focusContext(layoutData)
  const focusPresentationActive = focus.active
  // Surrounding context (annotations, other pages, groups) is hidden while a
  // focus session rests with the eye off; a working tool or the focus-bar eye
  // latches it on (ADR 0021). Binary show/hide, never a dim.
  const hideContext = focus.active && !focus.showsContext
  const pointerOwnerState = {
    toolKind: layoutData.activeTool.kind,
    pendingPlacement: Boolean(layoutData.pendingPlacement),
    pendingAnnotation: Boolean(pendingAnnotation),
    pendingRegionRect: Boolean(pendingRegionRect),
    openThread: Boolean(openThreadId),
    drawingSession: Boolean(drawingSession),
  }
  const overlayInteractive = annotationOverlayActive(pointerOwnerState)
  const pointerOwner = canvasPointerOwner(pointerOwnerState)
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
    handleOverlayPointerCancel,
    handleOverlayPointerDown,
    handleOverlayPointerMove,
    handleOverlayPointerUp,
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
  useCanvasFileDrop({ api, layoutRef })

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
      // rect so it aligns with the mouse.
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

  const hoverForwardingEnabled =
    layoutData.activeTool.kind !== 'draw' && layoutData.activeTool.kind !== 'comment'
  usePageInputForwarding({
    api,
    layoutRef,
    pendingPlacement,
    hoverForwardingEnabled,
    setPlacementCursor,
  })

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
      if (layout.interaction.kind !== 'idle') return false
      // In focus presentation the page isn't single-selected, but the wheel
      // should still scroll it instead of panning (which would exit focus).
      const focusedPageId = focusContext(layout).pageId
      const selected = layout.selectedEntityIds
      const pageId = focusedPageId ?? (selected.length === 1 ? selected[0] : null)
      if (!pageId) return false
      const page = layout.entities.find(
        (entity): entity is CanvasSceneEntity & { kind: 'page' } =>
          entity.kind === 'page' && entity.id === pageId,
      )
      if (!page) return false
      const windowY = clientYToWindowY(event.clientY, layout)
      // In focus presentation the camera is locked on the page, so any wheel
      // scrolls it — skip the cursor-over-body check used for selected pages.
      if (!focusedPageId && !pointerOverPageContent(page, { x: event.clientX, y: windowY })) {
        return false
      }
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
  // Let the wheel scroll a note's body natively instead of panning the canvas.
  // Scoped to the single selected (preview) or actively-edited entity, mirroring
  // the selected-page rule above — and only when its body actually overflows, so
  // wheeling over a short note still pans.
  const yieldWheelToEntityScroll = useCallback(
    (event: WheelEvent): boolean => {
      const layout = layoutRef.current
      const kind = layout.interaction.kind
      if (kind !== 'idle' && kind !== 'editing-entity') return false
      const editingId =
        layout.interaction.kind === 'editing-entity'
          ? layout.interaction.entityId
          : null
      const target = event.target
      if (!(target instanceof Element)) return false
      const shell = target.closest('[data-entity-id]')
      const entityId = shell?.getAttribute('data-entity-id')
      if (!entityId) return false
      const selected = layout.selectedEntityIds
      const active =
        editingId === entityId ||
        (selected.length === 1 && selected[0] === entityId)
      if (!active) return false
      return canScrollWheelTarget(target, event)
    },
    [layoutRef],
  )
  useViewportWheelAndMiddlePan(
    true,
    viewportWheelAndPanApi,
    routeWheel,
    yieldWheelToEntityScroll,
  )

  // ADR 0001 — canvas pointer router. Single window-level pointerdown
  // listener that runs the shared hit-test, classifies the action via the
  // priority table, and dispatches every gesture (focus, drag, resize,
  // edge-drag, marquee, pan) through the existing IPC surface. The
  // `EdgeDragLayer` below renders the rubber-band line driven by the same
  // controller state.
  const spaceHeldRef = useRef(false)
  const optionHeldRef = useRef(false)
  const commandHeldRef = useRef(false)
  const handToolActiveRef = useRef(layoutData.activeTool.kind === 'hand')
  handToolActiveRef.current = layoutData.activeTool.kind === 'hand'
  useEffect(() => {
    const onKey = (event: KeyboardEvent, down: boolean) => {
      if (event.code === 'Space') spaceHeldRef.current = down
      if (event.key === 'Alt' || event.code === 'AltLeft' || event.code === 'AltRight') {
        optionHeldRef.current = down
      }
      if (
        event.key === 'Meta' ||
        event.key === 'Control' ||
        event.code === 'MetaLeft' ||
        event.code === 'MetaRight' ||
        event.code === 'ControlLeft' ||
        event.code === 'ControlRight'
      ) {
        commandHeldRef.current = down
      }
    }
    const onDown = (e: KeyboardEvent) => onKey(e, true)
    const onUp = (e: KeyboardEvent) => onKey(e, false)
    const onBlur = () => {
      spaceHeldRef.current = false
      optionHeldRef.current = false
      commandHeldRef.current = false
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
  const [groupDropTargetId, setGroupDropTargetId] = useState<string | null>(null)
  const [dropBindingSuppressed, setDropBindingSuppressed] = useState(false)
  // Interactive file (HTML iframe) the user has entered: select-first /
  // interact-second, mirroring pages. Renderer-local — the iframe lives in
  // this WCV's DOM, so entering just flips its pointer-events (no cross-
  // process forwarding pages need). Cleared when it stops being the sole
  // selection (Escape → selectNone, click-away, select another).
  const [enteredEntityId, setEnteredEntityId] = useState<string | null>(null)
  const enteredEntityIdRef = useRef<string | null>(null)
  enteredEntityIdRef.current = enteredEntityId
  const onEnterEntityInteractive = useCallback((entityId: string) => {
    setEnteredEntityId(entityId)
  }, [])
  useEffect(() => {
    if (!enteredEntityId) return
    const sel = layoutData.selectedEntityIds
    if (sel.length !== 1 || sel[0] !== enteredEntityId) setEnteredEntityId(null)
  }, [enteredEntityId, layoutData.selectedEntityIds])
  useCanvasPointerRouter({
    api,
    layoutRef,
    owner: pointerOwner,
    consume: FULL_ROUTER_CONSUME,
    spaceHeldRef,
    handToolActiveRef,
    optionHeldRef,
    commandHeldRef,
    setDragCopyPreview,
    setGroupDropTarget: setGroupDropTargetId,
    setDropBindingSuppressed,
    setEdgeDragState,
    setReorderGhost,
    onCommentDragMove: onDragMove,
    onCommentDragEnd: onDragEnd,
    commentDraftRef: draftStateRef,
    enteredEntityIdRef,
    onEnterEntityInteractive,
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
    // Overlay UI (tool popups, chrome) keeps a normal pointer — its attribute
    // selector outspecifies `body *`, so the default cursor wins on hover.
    style.textContent = `html, body, body * { cursor: ${DRAW_CURSOR} !important; }
[data-overlay-ui], [data-overlay-ui] * { cursor: default !important; }`
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

  const popupContext: PopupContext = {
    api,
    isDark,
    layout: layoutData,
    interactionIdle,
    sameKindSelection,
    selectedGroup: selectedGroupEntity,
    textPopupReady,
  }

  return (
    // aboveView is the always-on canvas-mode input authority (I7): every
    // pointer-owner state keeps the root interactive; individual layers opt
    // out with pointer-events-none.
    <div
      className="pointer-events-auto relative h-screen w-screen overflow-hidden bg-transparent"
      style={{
        cursor: drawInteractionEnabled ? DRAW_CURSOR : undefined,
      }}
      onPointerDown={handleOverlayPointerDown}
      onPointerMove={handleOverlayPointerMove}
      onPointerUp={handleOverlayPointerUp}
      onPointerCancel={handleOverlayPointerCancel}
    >
      {/* Translate the whole canvas scene live with the pan gesture so selection
          chrome and entity bodies track the natively-positioned page views
          instead of trailing until the next layout-update rebuild (#257). Pan is
          disabled during focus, where the only viewport-pinned chrome exists, so
          every layer here is canvas-space and moves together. */}
      <div
        className="pointer-events-none absolute inset-0"
        style={{ transform: `translate3d(${panOffset.x}px, ${panOffset.y}px, 0)` }}
      >
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
            setCommentText={setCommentText}
            setElementNameDraft={setElementNameDraft}
            submitPendingAnnotation={submitPendingAnnotation}
            submitRegionAnnotation={submitRegionAnnotation}
          />

        </>
      ) : null}

      {/* Persistent entity bodies (drawings, stickies, notes, shapes, files,
          edges) render in capture mode too — region captures composite the real
          canvas. Only hover/interaction chrome below stays behind !captureMode.
          Debug CSS injected into pages is suppressed separately (capture-suppression). */}
      <StackedCanvasItems
        layoutData={renderLayout}
        hoveredEntityId={hoveredEntityId}
        isDark={isDark}
        selectedEdgeIds={selectedEdgeIds}
        selectedEntityIdSet={selectedEntityIdSet}
        editingEntityId={editingEntityId}
        interactiveEntityId={enteredEntityId}
        ghostEntity={reorderGhostEntity}
        hideContext={hideContext}
      />

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
            submitThreadReply={submitThreadReply}
            threadInputRef={threadInputRef}
            threadPosition={threadPosition}
          />

          <MarqueeLayer overlay={selectionOverlay} />

          {/* Live drawing preview renders after StackedCanvasItems so the
              in-progress stroke sits above file entities — matching where a
              freshly committed drawing lands at the top of the entity order. */}
          {drawingSession ? (
            <DrawingLayer
              drawing={{ version: 1, ...drawingSession }}
              layout={layoutData}
              active
              isDark={isDark}
            />
          ) : null}

          {/* Placement preview renders after StackedCanvasItems for the same
              reason the drawing preview does: the ghost sits above existing
              entity bodies, matching where the stamped item lands at the top of
              the entity order. Rendered earlier it paints beneath file entities
              and then jumps on top once placed. */}
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

          <EdgeLayer
            edges={[]}
            entities={layoutData.entities}
            hoveredEntityId={hoveredEntityId}
            isDark={isDark}
            interaction={layoutData.interaction}
            selectedEdgeIds={selectedEdgeIds}
            selectedEntityIds={focusPresentationActive ? [] : layoutData.selectedEntityIds}
            zoom={layoutData.zoom}
            originY={layoutData.canvasOrigin.y}
            onSelectEdge={api.selectEdge}
            renderAnchors={!focusPresentationActive}
          />

          {(layoutData.groups?.length ?? 0) > 0 && !hideContext ? (
            <GroupBoundsLayer
              groups={renderLayout.groups ?? []}
              isDark={isDark}
              zoom={layoutData.zoom}
              canvasOrigin={layoutData.canvasOrigin}
              pan={layoutData.pan}
              dropTargetGroupId={groupDropTargetId}
            />
          ) : null}

          {!focusPresentationActive ? (
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

          {/* Render during a focus session too — only the focused page's own box
              is suppressed (clean read); other items keep selection/hover
              outlines so eye-revealed annotations stay interactive (ADR 0021). */}
          <SelectionOutlineLayer
            layoutData={renderLayout}
            isDark={isDark}
            marqueePreviewIds={marqueePreviewIds}
            reorderGhostId={reorderGhostEntity?.id ?? null}
            reorderGhostSpan={reorderGhostSpan}
            suppressPageId={focus.pageId}
            suppressPageHover={dropBindingSuppressed}
          />

          <EdgeDragLayer state={edgeDragState} layoutData={layoutData} isDark={isDark} />
          <DragCopyPreviewLayer previews={dragCopyPreview} isDark={isDark} />
          <GuideOverlayLayer guides={canvasGuides} layoutData={layoutData} isDark={isDark} />

          <GapHandlesLayer layoutData={renderLayout} />
          <ReorderDotsLayer layoutData={renderLayout} />

          <GroupRenameOverlay
            api={api}
            layoutData={layoutData}
            isDark={isDark}
            editingEntityId={editingEntityId}
            optionHeldRef={optionHeldRef}
            commandHeldRef={commandHeldRef}
            setDragCopyPreview={setDragCopyPreview}
            setGroupDropTarget={setGroupDropTargetId}
            setDropBindingSuppressed={setDropBindingSuppressed}
          />

          {/* Tool-vs-selection mutex (ADR 0008 §2): the active tool's popup wins
              and suppresses the selection popups. PagePopup is exempt while a
              focus session is active — it doubles as the focus bar, and
              ViewportAnchor stacks the tool popup below it. */}
          {TOOL_POPUPS.filter((row) => row.toolKind === layoutData.activeTool.kind).map(
            ({ toolKind, Component, extraProps }) => (
              <Component
                key={toolKind}
                api={api}
                isDark={isDark}
                layout={layoutData}
                {...extraProps}
              />
            ),
          )}
          {SELECTION_POPUPS.filter((row) =>
            row.focusExempt
              ? !toolHasPopup(layoutData.activeTool) || focusPresentationActive
              : !toolHasPopup(layoutData.activeTool),
          ).map(({ key, Component, mapProps }) => (
            <Component key={key} {...mapProps(popupContext)} />
          ))}

          {/* Edges aren't scene entities, so they sit outside SELECTION_POPUPS.
              Mount off the single selected edge, under the same tool mutex. */}
          {!toolHasPopup(layoutData.activeTool) && !hideContext ? (
            <EdgePopup
              key={selectedEdge?.id ?? 'none'}
              api={api}
              isDark={isDark}
              layout={layoutData}
              edge={selectedEdge}
            />
          ) : null}

          <CommentBadgesLayer
            annotations={layoutData.annotations}
            layoutData={layoutData}
            liveBboxes={liveBboxes}
            onOpenThread={openThreadById}
          />
        </>
      ) : null}
      </div>
    </div>
  )
}
