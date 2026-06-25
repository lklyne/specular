/**
 * Canvas pointer router — single source of arbitration for canvas pointer
 * input in canvas mode (ADR 0001).
 *
 * Runs the shared `hitTest` against the current layout snapshot on
 * pointerdown and dispatches a typed `CanvasPointerAction` to the existing
 * IPC surface. Replaces the per-layer `onMouseDown` handlers that used to
 * live in bgView (`PageChromeLayer`, `EdgeLayer`, `ResizeHandles`,
 * `EntityBlockLayers`, `GroupBoundsLayer`, and the old mouse gesture hooks).
 *
 * The router runs entirely in the renderer because the layout snapshot it
 * needs (entities, edges, selection, zoom) is already broadcast to
 * aboveView via `layout-update`. Pure modules carry the logic:
 *
 *   - `src/shared/hit-test.ts` — priority-ordered hit classification.
 *   - `src/shared/canvas-pointer-actions.ts` — action descriptor.
 *   - `src/shared/edge-drag-controller.ts` — edge-drag state machine.
 *   - `src/shared/resize-accumulator.ts` — resize math.
 *
 * The router itself owns only the wiring: window-level pointer listeners,
 * per-action drag-installation, IPC dispatch, and renderer-local visual
 * state for the edge-drag rubber-band.
 */

import { useEffect, useRef } from 'react'
import { hitTest, type HitInputs } from '../../shared/hit-test'
import {
  routePointerDoubleClick,
  routePointerDown,
  type CanvasPointerAction,
  type CanvasPointerContext,
} from '../../shared/canvas-pointer-actions'
import {
  beginEdgeDrag as beginEdgeDragState,
  cancelEdgeDrag as cancelEdgeDragState,
  commitEdgeDrag as commitEdgeDragState,
  EDGE_DRAG_IDLE,
  updateEdgeDragCursor,
  type EdgeDragState,
} from '../../shared/edge-drag-controller'
import {
  applyHandleDelta,
  startResize,
  type AspectRatioResizeMode,
  type ResizeConfig,
} from '../../shared/resize-accumulator'
import { scaleStrokesToBounds } from '../../shared/scale-strokes'
import {
  applyMultiHandleDelta,
  computeMultiSelectionBbox,
  startMultiResize,
} from '../../shared/multi-resize-accumulator'
import {
  entitiesOverlappingRect,
  DRAG_THRESHOLD,
  isOverlayUiTarget,
  isTypingTarget,
  middleDragDelta,
  normalizeRect,
  screenPointToCanvasPoint,
  screenRectToCanvasRect,
} from '../../shared/gesture-utils'
import { aspectRatioResizeModeForCanvasFile } from '../canvas-bg/entityConstants'
import {
  MIN_FILE_HEIGHT,
  MIN_FILE_WIDTH,
  MIN_GROUP_HEIGHT,
  MIN_GROUP_WIDTH,
  MIN_SHAPE_HEIGHT,
  MIN_SHAPE_WIDTH,
  MIN_TEXT_HEIGHT,
  MIN_TEXT_WIDTH,
} from '../canvas-bg/entityConstants'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import type {
  CanvasBgElectronAPI,
  CanvasSceneEntity,
  EdgeSide,
  LayoutUpdateData,
  SelectionModifiers,
} from '../../shared/types'
import {
  startOptionAwareEntityDrag,
  startOptionAwareGroupDrag,
  type DragCopyPreviewBox,
} from './optionDragCopy'

interface UseCanvasPointerRouterOptions {
  api: CanvasBgElectronAPI
  layoutRef: React.MutableRefObject<LayoutUpdateData>
  /** When false, the router does not intercept anything. Useful while
   *  annotations / drawing own pointer input. */
  enabled: boolean
  /** Hit kinds the router should consume. */
  consume: ReadonlySet<CanvasPointerAction['kind']>
  /** Space-modifier mirror — `useCanvasPointerRouter` reads this on each
   *  pointerdown to decide pan-on-background. */
  spaceHeldRef: React.MutableRefObject<boolean>
  /** Hand-tool mirror — when the toolbar's hand tool is active, pan-on-
   *  background fires regardless of space. ADR 0013 §5 nav group. */
  handToolActiveRef: React.MutableRefObject<boolean>
  /** Option-modifier mirror. Pointer events can miss mid-drag key changes,
   *  so drag-copy reads this live throughout the gesture. */
  optionHeldRef: React.MutableRefObject<boolean>
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void
  /** Edge-drag visual state setter. The router updates this so
   *  `EdgeDragLayer` can render the rubber-band. */
  setEdgeDragState: (state: EdgeDragState) => void
  /** Reorder-ghost offset setter (ADR 0015 D7, Phase D). The router publishes
   *  the live canvas-space pointer delta since grab so App can float the
   *  dragged item under the cursor as a ghost. Null when not reordering. */
  setReorderGhost: (ghost: ReorderGhostOffset) => void
}

/** Canvas-space pointer delta since a reorder grab — drives the floating ghost.
 *  Null outside a reorder drag (ADR 0015 D7, Phase D). */
export type ReorderGhostOffset = { dx: number; dy: number } | null

const ALL_KINDS: ReadonlySet<CanvasPointerAction['kind']> = new Set<CanvasPointerAction['kind']>([
  'noop',
  'page-body-press',
  'forward-pointer-down',
  'begin-entity-drag',
  'begin-entity-press',
  'begin-group-drag',
  'begin-resize',
  'begin-multi-resize',
  'begin-edge-drag',
  'toggle-select',
  'background-click',
  'begin-marquee',
  'begin-pan',
  'begin-reorder-drag',
])

/** All routable kinds — used by tests and any caller that wants full
 *  router authority. Production aboveView passes this, making the router
 *  the canvas-mode authority for selection, drag, resize, marquee, pan,
 *  and edge gestures. */
export const FULL_ROUTER_CONSUME = ALL_KINDS

function layoutToHitInputs(layout: { entities: HitInputs['entities']; edges?: HitInputs['edges'] | null; selectedEntityIds: HitInputs['selectedEntityIds']; selectedGroupId?: string | null; hover?: { id: string } | null; zoom?: number | null }): HitInputs {
  return {
    entities: layout.entities,
    edges: layout.edges ?? [],
    selectedEntityIds: layout.selectedEntityIds,
    selectedGroupId: layout.selectedGroupId ?? null,
    hoveredEntityId: layout.hover?.id ?? null,
    zoom: layout.zoom ?? 1,
  }
}

function capturePointer(event: PointerEvent): (() => void) | null {
  const target = event.target
  if (!(target instanceof Element)) return null
  try {
    target.setPointerCapture(event.pointerId)
  } catch {
    return null
  }
  return () => {
    try {
      if (target.hasPointerCapture(event.pointerId)) {
        target.releasePointerCapture(event.pointerId)
      }
    } catch {
      /* ignore */
    }
  }
}

export function useCanvasPointerRouter(options: UseCanvasPointerRouterOptions): void {
  const {
    api,
    layoutRef,
    enabled,
    consume,
    spaceHeldRef,
    handToolActiveRef,
    optionHeldRef,
    setDragCopyPreview,
    setEdgeDragState,
    setReorderGhost,
  } = options
  const apiRef = useRef(api)
  apiRef.current = api
  const consumeRef = useRef(consume)
  consumeRef.current = consume
  const setEdgeDragStateRef = useRef(setEdgeDragState)
  setEdgeDragStateRef.current = setEdgeDragState

  useEffect(() => {
    if (!enabled) return

    const handlePointerDown = (event: PointerEvent) => {
      if (isOverlayUiTarget(event.target)) return
      // Yield to typing targets (textarea, input, contenteditable) so focus
      // and cursor positioning land normally. Without this, the router's
      // preventDefault on entity-body hits eats the click before the
      // editable element can react — affects sticky textareas, markdown /
      // wireframe-JSON file textareas, and any future inline editor.
      if (isTypingTarget(event.target)) return
      if (event.button !== 0 && event.button !== 1 && event.button !== 2) return

      const layout = layoutRef.current

      // aboveView's WCV starts at canvasOrigin.y; scene entities use
      // window-relative screenY, so add the offset before hit-testing.
      const windowY = event.clientY + layout.canvasOrigin.y
      const inputs = layoutToHitInputs(layout)
      const target = hitTest(inputs, { x: event.clientX, y: windowY })

      // Inline-edit outside-click (primary button) commits the active edit
      // and falls through to normal routing, so the same click also acts
      // (deselect on background, switch selection on another entity, etc.)
      // — two clicks to enter edit, one click to exit. This differs from
      // ADR 0001's frame-focus rule (two clicks to act after focus) because
      // inline entity edits don't capture native input the way focused
      // frames do. Right/middle clicks ignore edit mode entirely so context
      // menus and middle-click pan keep working.
      const editingEntityId =
        layout.interaction.kind === 'editing-entity'
          ? layout.interaction.entityId
          : null
      if (editingEntityId !== null && event.button === 0) {
        const hitEntityId =
          target.payload.kind === 'entity-body' ||
          target.payload.kind === 'page-body' ||
          target.payload.kind === 'chrome' ||
          target.payload.kind === 'resize-handle' ||
          target.payload.kind === 'anchor'
            ? target.payload.entityId
            : null
        if (hitEntityId !== editingEntityId) {
          apiRef.current.commitEntityEdit()
        }
      }

      const modifiers: SelectionModifiers = {
        shift: event.shiftKey,
        meta: event.metaKey,
        ctrl: event.ctrlKey,
      }
      const context: CanvasPointerContext = {
        selectedEntityIds: layout.selectedEntityIds,
        isPrimaryButton: event.button === 0,
        button: event.button === 1 ? 'middle' : event.button === 2 ? 'right' : 'left',
        modifiers,
        spaceHeld: spaceHeldRef.current || handToolActiveRef.current,
        altHeld: event.altKey || optionHeldRef.current,
        editingEntityId,
      }

      // Hand tool: primary-button drag pans globally regardless of hit
      // target. Space-held still defers to the routing matrix (pan on
      // background, modifier elsewhere) — hand tool is the explicit
      // pan-only mode that suppresses entity drag/resize/edge gestures.
      const action: CanvasPointerAction =
        handToolActiveRef.current && event.button === 0
          ? { kind: 'begin-pan' }
          : routePointerDown(target, context)
      if (!consumeRef.current.has(action.kind)) return

      const dispatched = dispatchAction({
        action,
        api: apiRef.current,
        event,
        layoutRef,
        optionHeldRef,
        setDragCopyPreview,
        setEdgeDragState: setEdgeDragStateRef.current,
        setReorderGhost,
      })
      if (dispatched) {
        event.preventDefault()
        event.stopPropagation()
      }
    }

    const handleDblClick = (event: MouseEvent) => {
      if (isOverlayUiTarget(event.target)) return
      if (isTypingTarget(event.target)) return
      if (event.button !== 0) return
      const layout = layoutRef.current
      const windowY = event.clientY + layout.canvasOrigin.y
      const target = hitTest(layoutToHitInputs(layout), { x: event.clientX, y: windowY })
      const action = routePointerDoubleClick(target)
      switch (action.kind) {
        case 'noop':
          return
        case 'request-entity-edit':
          apiRef.current.requestEntityEdit(action.entityId)
          break
        case 'enter-group':
          apiRef.current.enterGroup(action.groupId)
          break
        case 'enter-group-rename':
          // GroupRenameLabel handles the dblclick directly via the DOM
          // (it's tagged data-overlay-ui so isOverlayUiTarget catches it
          // earlier — this branch only fires if the rename label is hit
          // through the chrome slot via hit-test).
          return
      }
      event.preventDefault()
      event.stopPropagation()
    }

    window.addEventListener('pointerdown', handlePointerDown, { capture: true })
    window.addEventListener('dblclick', handleDblClick, { capture: true })
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown, {
        capture: true,
      } as EventListenerOptions)
      window.removeEventListener('dblclick', handleDblClick, {
        capture: true,
      } as EventListenerOptions)
    }
  }, [enabled, handToolActiveRef, layoutRef, optionHeldRef, setDragCopyPreview, setReorderGhost, spaceHeldRef])
}

// --- Dispatch ---

interface DispatchContext {
  action: CanvasPointerAction
  api: CanvasBgElectronAPI
  event: PointerEvent
  layoutRef: React.MutableRefObject<LayoutUpdateData>
  optionHeldRef: React.MutableRefObject<boolean>
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void
  setEdgeDragState: (state: EdgeDragState) => void
  setReorderGhost: (ghost: ReorderGhostOffset) => void
}

function dispatchAction(ctx: DispatchContext): boolean {
  const { action, api, event, layoutRef, optionHeldRef, setDragCopyPreview, setEdgeDragState, setReorderGhost } = ctx
  switch (action.kind) {
    case 'noop':
      return false
    case 'page-body-press':
      return runPageBodyPress(action, api, event, layoutRef, optionHeldRef, setDragCopyPreview)
    case 'forward-pointer-down':
      return runForwardPointer(action, api, event, layoutRef)
    case 'toggle-select':
      if (action.entityKind === 'page') {
        api.selectPage(action.entityId, { shift: true, meta: false, ctrl: false })
      } else if (action.entityKind === 'group') {
        api.selectGroup(action.entityId)
      } else {
        api.selectEntity(action.entityId, action.entityKind, {
          shift: true,
          meta: false,
          ctrl: false,
        })
      }
      return true
    case 'background-click':
      return runBackgroundSelectionGesture(api, event, layoutRef)
    case 'begin-entity-drag':
      return runEntityDrag(action, api, event, layoutRef, optionHeldRef, setDragCopyPreview)
    case 'begin-entity-press':
      return runEntityPress(action, api, event, layoutRef, optionHeldRef, setDragCopyPreview)
    case 'begin-group-drag':
      return runGroupDrag(action, api, event, layoutRef, optionHeldRef, setDragCopyPreview)
    case 'begin-resize':
      return runResize(action, api, event, layoutRef)
    case 'begin-multi-resize':
      return runMultiResize(action, api, event, layoutRef)
    case 'begin-edge-drag':
      return runEdgeDrag(action, api, event, layoutRef, setEdgeDragState)
    case 'begin-marquee':
      return runBackgroundSelectionGesture(api, event, layoutRef)
    case 'begin-pan':
      return runPan(api, event)
    case 'begin-reorder-drag':
      return runReorderDrag(action, api, event, layoutRef, setReorderGhost)
  }
}

// --- Per-action handlers ---

function runEntityDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-entity-drag' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
  optionHeldRef: React.MutableRefObject<boolean>,
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void,
): boolean {
  const releasePointer = capturePointer(event)
  startOptionAwareEntityDrag({
    api,
    layout: layoutRef.current,
    entityId: action.entityId,
    entityKind: action.entityKind,
    preserveSelection: action.preserveSelection,
    event,
    releasePointer,
    isOptionHeld: () => optionHeldRef.current,
    setPreview: setDragCopyPreview,
  })
  return true
}

function runEntityPress(
  action: Extract<CanvasPointerAction, { kind: 'begin-entity-press' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
  optionHeldRef: React.MutableRefObject<boolean>,
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void,
): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  const startScreenX = event.screenX
  const startScreenY = event.screenY
  let dragging = false

  const cleanup = () => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
  }

  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    if (!dragging) {
      const totalDx = ev.screenX - startScreenX
      const totalDy = ev.screenY - startScreenY
      if (
        Math.abs(totalDx) < DRAG_THRESHOLD &&
        Math.abs(totalDy) < DRAG_THRESHOLD
      ) {
        return
      }
      dragging = true
      cleanup()
      startOptionAwareEntityDrag({
        api,
        layout: layoutRef.current,
        entityId: action.entityId,
        entityKind: action.entityKind,
        preserveSelection: true,
        event,
        releasePointer,
        initialPointer: ev,
        isOptionHeld: () => optionHeldRef.current,
        setPreview: setDragCopyPreview,
      })
      return
    }
  }

  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
    if (dragging) {
      api.endDragEntity()
      return
    }
    api.requestEntityEdit(action.entityId)
  }

  const onCancel = (ev: Event) => {
    // Pre-threshold blur is a phantom: focus reconciliation routes focus
    // aboveView → bgView on the next layout pass after a prior gesture
    // ends. A second click landing inside that window would otherwise see
    // the armed press torn down before pointerup, dropping the edit.
    if (!dragging && ev.type === 'blur') return
    cleanup()
    if (dragging) api.endDragEntity()
  }

  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

function runPageBodyPress(
  action: Extract<CanvasPointerAction, { kind: 'page-body-press' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
  optionHeldRef: React.MutableRefObject<boolean>,
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void,
): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  const startScreenX = event.screenX
  const startScreenY = event.screenY
  let dragging = false

  const cleanup = () => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
  }

  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    const totalDx = ev.screenX - startScreenX
    const totalDy = ev.screenY - startScreenY
    if (
      !dragging &&
      Math.abs(totalDx) < DRAG_THRESHOLD &&
      Math.abs(totalDy) < DRAG_THRESHOLD
    ) {
      return
    }
    if (!dragging) {
      dragging = true
      cleanup()
      startOptionAwareEntityDrag({
        api,
        layout: layoutRef.current,
        entityId: action.entityId,
        entityKind: 'page',
        preserveSelection: action.preserveSelection,
        event,
        releasePointer,
        initialPointer: ev,
        isOptionHeld: () => optionHeldRef.current,
        setPreview: setDragCopyPreview,
      })
      return
    }
  }

  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
    if (dragging) {
      api.endDragPage()
      return
    }
    // Thread modifiers through so a shift/cmd-click on an unselected or
    // multi-selected page body extends the selection instead of replacing
    // it. Routing already converts additive clicks on page-body to
    // toggle-select, but reading the live modifier state here keeps the
    // gesture honest if the user presses shift between down and up.
    api.selectPage(action.entityId, {
      shift: ev.shiftKey,
      meta: ev.metaKey,
      ctrl: ev.ctrlKey,
    })
  }

  const onCancel = (ev: Event) => {
    // Pre-threshold blur is a phantom: focus reconciliation routes focus
    // aboveView → bgView on the next layout pass (debounced 16ms) after a
    // drag ends. A second click that lands inside that window installs
    // this listener, then the pending reconcile blurs aboveView before
    // any cursor movement — tearing the armed gesture down here would
    // kill the second drag with no recovery. Wait for actual movement;
    // pointerup / pointercancel still abort cleanly.
    if (!dragging && ev.type === 'blur') return
    cleanup()
    if (dragging) api.endDragPage()
  }

  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

function runGroupDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-group-drag' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
  optionHeldRef: React.MutableRefObject<boolean>,
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void,
): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  let dragging = false
  const startScreenX = event.screenX
  const startScreenY = event.screenY

  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    const totalDx = ev.screenX - startScreenX
    const totalDy = ev.screenY - startScreenY
    if (
      !dragging &&
      Math.abs(totalDx) < DRAG_THRESHOLD &&
      Math.abs(totalDy) < DRAG_THRESHOLD
    ) {
      return
    }
    if (!dragging) {
      dragging = true
      cleanup()
      startOptionAwareGroupDrag({
        api,
        layout: layoutRef.current,
        groupId: action.groupId,
        event,
        releasePointer,
        initialPointer: ev,
        isOptionHeld: () => optionHeldRef.current,
        setPreview: setDragCopyPreview,
      })
      return
    }
  }
  const cleanup = () => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
  }
  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
    if (dragging) {
      api.endDragGroup()
      return
    }
    api.selectGroup(action.groupId)
  }
  const onCancel = () => {
    cleanup()
    if (dragging) api.endDragGroup()
  }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

function runResize(
  action: Extract<CanvasPointerAction, { kind: 'begin-resize' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  const layout = layoutRef.current
  const entity = layout.entities.find((e) => e.id === action.entityId)
  if (!entity) return false
  const config = resizeConfigForEntity(entity)
  const acc = startResize({
    width: entity.width,
    height: entity.height,
    canvasX: entity.canvasX,
    canvasY: entity.canvasY,
  })
  const zoom = layout.zoom ?? 1
  const dispatchPatch = patchDispatcherForKind(entity.kind, action.entityId, api)
  if (!dispatchPatch) return false

  // For drawing entities, augment each patch with strokes transformed from the
  // initial bounds so absolute canvas-space stroke geometry tracks the resized
  // selection box in real time.
  const effectiveDispatch = entity.kind === 'drawing'
    ? (() => {
        const initialStrokes = entity.strokes
        const initialBounds = {
          canvasX: entity.canvasX,
          canvasY: entity.canvasY,
          width: entity.width,
          height: entity.height,
        }
        return (patch: { width: number; height: number; canvasX?: number; canvasY?: number }) => {
          const nextBounds = {
            canvasX: patch.canvasX ?? initialBounds.canvasX,
            canvasY: patch.canvasY ?? initialBounds.canvasY,
            width: patch.width,
            height: patch.height,
          }
          api.updateDrawingEntity(action.entityId, {
            ...patch,
            strokes: scaleStrokesToBounds(initialStrokes, initialBounds, nextBounds),
          })
        }
      })()
    : dispatchPatch

  // Plain text in 'auto' widthMode is content-driven; the renderer's
  // ResizeObserver overwrites any width/height we'd dispatch. Flip to
  // 'fixed' first so the upcoming width/height patches stick.
  if (entity.kind === 'text' && entity.widthMode === 'auto') {
    api.updateTextEntity(action.entityId, { widthMode: 'fixed' })
  }

  // Enter resize mode in main BEFORE the first dispatchPatch. The bounds-update
  // IPC synchronously requestLayouts; if interactionState is still 'idle' when
  // reconcileFocus runs, focus moves to the selected page (pages only — they
  // populate focusedPageId), aboveView blurs, and the gesture is cancelled
  // after a single tick. Same gotcha as drag-start ordering.
  api.beginResize(action.entityId, entity.kind, action.handle)

  let lastX = event.screenX
  let lastY = event.screenY
  const cleanup = () => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
    api.endResize()
  }
  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    const screenDx = ev.screenX - lastX
    const screenDy = ev.screenY - lastY
    lastX = ev.screenX
    lastY = ev.screenY
    const patch = applyHandleDelta(
      acc,
      action.handle,
      { screenDx, screenDy, zoom, shiftKey: ev.shiftKey },
      config,
    )
    effectiveDispatch(patch)
  }
  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
  }
  const onCancel = () => cleanup()
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

function runMultiResize(
  action: Extract<CanvasPointerAction, { kind: 'begin-multi-resize' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  const layout = layoutRef.current
  const seed = computeMultiSelectionBbox(layout.entities, layout.selectedEntityIds)
  if (!seed) return false
  const acc = startMultiResize(seed)
  const zoom = layout.zoom ?? 1

  // Enter multi-resize interaction state in main BEFORE the first
  // resizeMultiSelection dispatch — same ordering requirement as single-entity
  // resize (see runResize comment above). Also opens a batch so all per-tick
  // mutations coalesce into one Y.Doc transaction / one undo step.
  api.beginMultiResize()

  let lastX = event.screenX
  let lastY = event.screenY
  const cleanup = () => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
    api.endMultiResize()
  }
  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    const screenDx = ev.screenX - lastX
    const screenDy = ev.screenY - lastY
    lastX = ev.screenX
    lastY = ev.screenY
    const entries = applyMultiHandleDelta(acc, action.handle, { screenDx, screenDy, zoom })
    api.resizeMultiSelection(entries)
  }
  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
  }
  const onCancel = () => cleanup()
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

function runEdgeDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-edge-drag' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
  setEdgeDragState: (state: EdgeDragState) => void,
): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  const layout = layoutRef.current
  const windowY = event.clientY + layout.canvasOrigin.y
  const entityMap = new Map<string, CanvasSceneEntity>()
  for (const e of layout.entities) entityMap.set(e.id, e)
  let state = beginEdgeDragState(
    action.entityId,
    action.side as EdgeSide,
    event.clientX,
    windowY,
    layout.edges ?? [],
    entityMap,
  )
  setEdgeDragState(state)

  // Tell main about the gesture begin so its interaction-controller is in
  // the right mode — this is what `EdgeLayer.tsx` used to call.
  const dragOriginEntityId =
    state.kind === 'edit' ? state.fixedEntityId : action.entityId
  const dragOriginSide =
    state.kind === 'edit' ? state.fixedSide : (action.side as EdgeSide)
  api.beginEdgeDrag(dragOriginEntityId, dragOriginSide)

  let lastSnap: string | null = null
  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    const cur = layoutRef.current
    const snapMap = new Map<string, CanvasSceneEntity>()
    for (const e of cur.entities) snapMap.set(e.id, e)
    const winY = ev.clientY + cur.canvasOrigin.y
    state = updateEdgeDragCursor(state, ev.clientX, winY, snapMap, cur.zoom ?? 1)
    setEdgeDragState(state)
    const snapKey = state.kind !== 'idle' && state.snap
      ? `${state.snap.entityId}:${state.snap.side}`
      : null
    if (snapKey !== lastSnap) {
      lastSnap = snapKey
      const target =
        state.kind !== 'idle' && state.snap
          ? { entityId: state.snap.entityId, side: state.snap.side }
          : null
      api.updateEdgeDragTarget(target?.entityId ?? null, target?.side ?? null)
    }
  }

  const finish = (mode: 'commit' | 'cancel') => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
    const outcome =
      mode === 'commit' ? commitEdgeDragState(state) : cancelEdgeDragState(state)
    switch (outcome.kind) {
      case 'create-edge':
        api.commitEdgeDrag(
          outcome.fromEntityId,
          outcome.toEntityId,
          outcome.fromSide,
          outcome.toSide,
        )
        break
      case 'edit-edge':
        api.commitEdgeEdit(
          outcome.edgeId,
          outcome.movingEnd,
          outcome.targetEntityId,
          outcome.targetSide,
        )
        break
      case 'discard-edge':
        api.discardEdgeEdit(outcome.edgeId)
        break
      case 'noop':
        api.cancelEdgeDrag()
        break
    }
    api.updateEdgeDragTarget(null, null)
    setEdgeDragState(EDGE_DRAG_IDLE)
  }

  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    finish('commit')
  }
  const onCancel = () => finish('cancel')
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

/**
 * Background drag → marquee.
 */
function runBackgroundSelectionGesture(
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
): boolean {
  const startClientX = event.clientX
  const startClientY = event.clientY
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  let dragged = false

  const cleanup = () => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
  }

  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    if (!dragged) {
      const dx = ev.clientX - startClientX
      const dy = ev.clientY - startClientY
      if (Math.abs(dx) < DRAG_THRESHOLD && Math.abs(dy) < DRAG_THRESHOLD) return
      dragged = true
    }
    const layout = layoutRef.current
    const rect = normalizeRect(startClientX, startClientY, ev.clientX, ev.clientY)
    const windowRect = {
      left: rect.left,
      top: rect.top + layout.canvasOrigin.y,
      width: rect.width,
      height: rect.height,
    }
    const entityIds = entitiesOverlappingRect(layout.entities, windowRect)
    api.setSelectionOverlayRect({
      rect: {
        ...rect,
        top: rect.top + (layout.canvasOrigin.y - TOOLBAR_HEIGHT),
      },
      variant: 'default',
      entityIds,
    })
  }
  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
    api.setSelectionOverlayRect(null)
    const layout = layoutRef.current
    const modifiers: SelectionModifiers = {
      shift: ev.shiftKey,
      meta: ev.metaKey,
      ctrl: ev.ctrlKey,
    }
    if (!dragged) {
      api.canvasDeselect(modifiers)
      return
    }
    const rect = normalizeRect(startClientX, startClientY, ev.clientX, ev.clientY)
    if (rect.width < 4 || rect.height < 4) {
      api.canvasDeselect(modifiers)
      return
    }
    const windowRect = { ...rect, top: rect.top + layout.canvasOrigin.y }
    api.canvasSelectInRect(screenRectToCanvasRect(windowRect, layout), modifiers)
  }
  const onCancel = () => {
    cleanup()
    api.setSelectionOverlayRect(null)
  }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

function runForwardPointer(
  action: Extract<CanvasPointerAction, { kind: 'forward-pointer-down' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  const { entityId, button } = action
  let lastWindowX = event.clientX
  let lastWindowY = event.clientY + layoutRef.current.canvasOrigin.y
  api.forwardPointerToPage(entityId, {
    kind: 'down',
    windowX: lastWindowX,
    windowY: lastWindowY,
    button,
    clickCount: event.detail || 1,
    shiftKey: event.shiftKey,
    ctrlKey: event.ctrlKey,
    altKey: event.altKey,
    metaKey: event.metaKey,
  })

  // Important: do NOT register a window `blur` listener here. Forwarding
  // `mouseDown` causes the focus-reconciler to move webContents focus to the
  // target page, which fires `blur` on aboveView. If we treated that as a
  // cancel, we'd tear down the gesture before `pointerup` arrives — leaving
  // the page stuck with a phantom mouseDown and the next click looking like
  // a release+drag rather than a fresh click.
  const cleanup = () => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
  }
  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    lastWindowX = ev.clientX
    lastWindowY = ev.clientY + layoutRef.current.canvasOrigin.y
    api.forwardPointerToPage(entityId, {
      kind: 'move',
      windowX: lastWindowX,
      windowY: lastWindowY,
      button,
      shiftKey: ev.shiftKey,
      ctrlKey: ev.ctrlKey,
      altKey: ev.altKey,
      metaKey: ev.metaKey,
    })
  }
  const sendUp = (ev: PointerEvent | null) => {
    const winX = ev ? ev.clientX : lastWindowX
    const winY = ev ? ev.clientY + layoutRef.current.canvasOrigin.y : lastWindowY
    api.forwardPointerToPage(entityId, {
      kind: 'up',
      windowX: winX,
      windowY: winY,
      button,
      clickCount: ev?.detail || 1,
      shiftKey: ev?.shiftKey ?? false,
      ctrlKey: ev?.ctrlKey ?? false,
      altKey: ev?.altKey ?? false,
      metaKey: ev?.metaKey ?? false,
    })
  }
  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
    sendUp(ev)
  }
  const onCancel = () => {
    cleanup()
    // Always release the page's mouseDown state so a canceled gesture
    // doesn't leak a stuck button.
    sendUp(null)
  }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  return true
}

function runPan(api: CanvasBgElectronAPI, event: PointerEvent): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)
  let lastScreenX = event.screenX
  let lastScreenY = event.screenY
  const cleanup = () => {
    releasePointer?.()
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
  }
  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    const { deltaX, deltaY } = middleDragDelta(
      { screenX: lastScreenX, screenY: lastScreenY },
      ev,
    )
    lastScreenX = ev.screenX
    lastScreenY = ev.screenY
    if (deltaX !== 0 || deltaY !== 0) api.canvasPan(deltaX, deltaY)
  }
  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
  }
  const onCancel = () => cleanup()
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

function runReorderDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-reorder-drag' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: React.MutableRefObject<LayoutUpdateData>,
  setReorderGhost: (ghost: ReorderGhostOffset) => void,
): boolean {
  const pointerId = event.pointerId
  const releasePointer = capturePointer(event)

  // Freeze the grab point so the ghost can float at original-pos + (live -
  // grab) — the grab offset is preserved, keeping the centre dot under the
  // pointer. Canvas-space so it survives pan/zoom mid-drag (it shouldn't, but
  // the math is origin-independent either way).
  const startLayout = layoutRef.current
  const grab = screenPointToCanvasPoint(
    event.clientX,
    event.clientY + startLayout.canvasOrigin.y,
    startLayout,
  )

  // Enter reorder mode in main BEFORE any layout-triggering work — same
  // gesture-begin ordering as resize/drag (see runtime/CLAUDE.md). With the
  // mode set to 'reordering-row', the focus reconciler keeps aboveView
  // focused, so the window-blur cancel below doesn't fire on the first tick.
  api.beginReorderDrag(action.movingId)
  // Lift the item the instant it's grabbed (50% ghost in place), before any move.
  setReorderGhost({ dx: 0, dy: 0 })

  const cleanup = () => {
    releasePointer?.()
    setReorderGhost(null)
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('blur', onCancel)
  }
  const onMove = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    const layout = layoutRef.current
    const point = screenPointToCanvasPoint(ev.clientX, ev.clientY + layout.canvasOrigin.y, layout)
    api.reorderDragMove(point.x, point.y)
    setReorderGhost({ dx: point.x - grab.x, dy: point.y - grab.y })
  }
  const onUp = (ev: PointerEvent) => {
    if (ev.pointerId !== pointerId) return
    cleanup()
    api.reorderDragCommit()
  }
  const onCancel = () => {
    cleanup()
    api.reorderDragCancel('blur')
  }
  window.addEventListener('pointermove', onMove)
  window.addEventListener('pointerup', onUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('blur', onCancel)
  return true
}

// --- Per-kind helpers ---

function resizeConfigForEntity(entity: CanvasSceneEntity): ResizeConfig {
  switch (entity.kind) {
    case 'page':
      return { minWidth: 320, minHeight: 200, aspectRatioResizeMode: 'off' }
    case 'group':
      return {
        minWidth: MIN_GROUP_WIDTH,
        minHeight: MIN_GROUP_HEIGHT,
        aspectRatioResizeMode: 'off',
      }
    case 'text':
      return {
        minWidth: MIN_TEXT_WIDTH,
        minHeight: MIN_TEXT_HEIGHT,
        aspectRatioResizeMode: 'off',
      }
    case 'file': {
      const aspect: AspectRatioResizeMode =
        'file' in entity && typeof entity.file === 'string'
          ? aspectRatioResizeModeForCanvasFile(entity.file)
          : 'off'
      return { minWidth: MIN_FILE_WIDTH, minHeight: MIN_FILE_HEIGHT, aspectRatioResizeMode: aspect }
    }
    case 'shape':
      return {
        minWidth: MIN_SHAPE_WIDTH,
        minHeight: MIN_SHAPE_HEIGHT,
        aspectRatioResizeMode: 'shift-locks',
      }
    case 'drawing':
      return { minWidth: 16, minHeight: 16, aspectRatioResizeMode: 'off' }
  }
}

function patchDispatcherForKind(
  kind: CanvasSceneEntity['kind'],
  id: string,
  api: CanvasBgElectronAPI,
): ((patch: { width: number; height: number; canvasX?: number; canvasY?: number }) => void) | null {
  switch (kind) {
    case 'page':
      return (patch) => api.updatePageBounds(id, patch)
    case 'group':
      return (patch) => api.updateGroupEntity(id, patch)
    case 'text':
      return (patch) => api.updateTextEntity(id, patch)
    case 'file':
      return (patch) => api.updateFileEntity(id, patch)
    case 'shape':
      return (patch) => api.updateShapeEntity(id, patch)
    case 'drawing':
      return (patch) => api.updateDrawingEntity(id, patch)
    default:
      return null
  }
}
