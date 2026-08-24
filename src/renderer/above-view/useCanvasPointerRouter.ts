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
 * state for the edge-drag rubber-band. Per-gesture drag sessions (pointer
 * capture, pointerId-filtered window listeners, teardown, blur handling)
 * come from `./pointer-session.ts`.
 */

import type { LayoutSnapshotRef } from '../shared/hooks/useProjectedLayoutRef'
import type { ProjectedLayoutData, ProjectedSceneEntity } from '../../shared/scene-projection'
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
  edgeDragOrigin,
  updateEdgeDragCursor,
  type EdgeDragState,
} from '../../shared/edge-drag-controller'
import {
  beginPressGesture,
  pressGestureIgnoresBlur,
  pressGestureStep,
  type PressGestureState,
} from '../../shared/press-gesture'
import {
  applyHandleDelta,
  startResize,
  type AspectRatioResizeMode,
  type ResizeConfig,
} from '../../shared/resize-accumulator'
import { scaleStrokesToBounds } from '../../shared/scale-strokes'
import { TEXT_SIZE_DEFAULT } from './TextSizeDropdown'
import { stickyResizePatch } from './stickyResize'
import {
  applyMultiHandleDelta,
  computeMultiSelectionBbox,
  startMultiResize,
} from '../../shared/multi-resize-accumulator'
import {
  canvasToScreenX,
  canvasToScreenY,
  clientYToWindowY,
  DRAG_THRESHOLD,
  entitiesOverlappingRect,
  isOverlayUiTarget,
  isTypingTarget,
  middleDragDelta,
  normalizeRect,
  screenPointToCanvasPoint,
  screenRectToCanvasRect,
  snapToGrid,
  squareConstrainedRect,
} from '../../shared/gesture-utils'
import type { MarqueeSelectionMode } from '../../shared/marquee-selection'
import type { CanvasPointerOwner } from '../../shared/canvas-pointer-owner'
import { aspectRatioResizeModeForCanvasFile } from '../canvas-bg/entityConstants'
import { ENTITY_KIND_CAPS } from '../../shared/entity-kind-caps'
import { TOOLBAR_HEIGHT } from '../../shared/constants'
import { focusContext } from '../../shared/focus-context'
import { runtimeStore } from '../shared/runtime-store'
import { GROUP_LABEL_FONT } from '../../shared/group-label-geometry'
import type { EdgeSide, SelectionModifiers } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import {
  startOptionAwareEntityDrag,
  startOptionAwareGroupDrag,
  type DragCopyPreviewBox,
} from './optionDragCopy'
import { capturePointer, startPointerSession } from './pointer-session'

export function commitInlineEditBeforePointerAction(
  blurActiveEditor: () => void,
  commitEntityEdit: () => void,
): void {
  // Pointer-down arrives before the browser's native blur. Force the blur
  // synchronously so the editor's onBlur commits its draft before main is
  // told to leave edit mode, rather than a tick later on the way out.
  blurActiveEditor()
  commitEntityEdit()
}

/** Live draft snapshot the comment gesture consults on pointerup — a click
 *  away from an empty composer dismisses it instead of opening a new one. */
interface CommentDraftSnapshot {
  pendingAnnotation: object | null
  pendingRegionRect: object | null
  commentText: string
  clearDraft: () => void
}

interface PointerDispatchDependencies {
  api: CanvasBgElectronAPI
  layoutRef: LayoutSnapshotRef
  optionHeldRef: React.MutableRefObject<boolean>
  commandHeldRef: React.MutableRefObject<boolean>
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void
  setGroupDropTarget: (groupId: string | null) => void
  setDropBindingSuppressed: (suppressed: boolean) => void
  setEdgeDragState: (state: EdgeDragState) => void
  setReorderGhost: (ghost: ReorderGhostOffset) => void
  onCommentDragMove: (startX: number, startY: number, endX: number, endY: number) => void
  onCommentDragEnd: (startX: number, startY: number, endX: number, endY: number) => void
  commentDraftRef: React.MutableRefObject<CommentDraftSnapshot>
  onEnterEntityInteractive: (entityId: string) => void
}

interface UseCanvasPointerRouterOptions extends PointerDispatchDependencies {
  /** Who owns canvas pointerdowns (`canvasPointerOwner`). 'router' runs the
   *  hit-test + routing matrix; 'tool-gesture' captures every canvas
   *  pointerdown for the active placement / comment tool; anything else
   *  stands the router down (annotations / drawing own pointer input). */
  owner: CanvasPointerOwner
  /** Hit kinds the router should consume. */
  consume: ReadonlySet<CanvasPointerAction['kind']>
  /** Space-modifier mirror — `useCanvasPointerRouter` reads this on each
   *  pointerdown to decide pan-on-background. */
  spaceHeldRef: React.MutableRefObject<boolean>
  /** Hand-tool mirror — when the toolbar's hand tool is active, pan-on-
   *  background fires regardless of space. ADR 0013 §5 nav group. */
  handToolActiveRef: React.MutableRefObject<boolean>
  /** The interactive file entity (HTML iframe) the user has entered, mirrored
   *  as a ref so each pointerdown reads it live. Renderer-owned state. */
  enteredEntityIdRef: React.MutableRefObject<string | null>
  /** Enter interactivity on an interactive file (the select-first / interact-
   *  second second click, or a double-click). Sets the renderer-local
   *  entered id, flipping the iframe's pointer-events on. */
  onEnterEntityInteractive: (entityId: string) => void
}

interface RouterPointerDependencies extends PointerDispatchDependencies {
  consume: ReadonlySet<CanvasPointerAction['kind']>
  spaceHeld: boolean
  handToolActive: boolean
  enteredEntityId: string | null
}

/** Canvas-space pointer delta since a reorder grab — drives the floating ghost.
 *  Null outside a reorder drag (ADR 0015 D7, Phase D). */
export type ReorderGhostOffset = { dx: number; dy: number } | null

const ALL_KINDS: ReadonlySet<CanvasPointerAction['kind']> = new Set<CanvasPointerAction['kind']>([
  'noop',
  'page-body-press',
  'enter-page-interactive',
  'enter-entity-interactive',
  'forward-pointer-down',
  'begin-entity-drag',
  'begin-entity-press',
  'begin-group-drag',
  'begin-resize',
  'begin-multi-resize',
  'begin-edge-drag',
  'toggle-select',
  'group-background-press',
  'background-click',
  'begin-marquee',
  'begin-pan',
  'begin-reorder-drag',
  'begin-gap-drag',
  'begin-placement',
  'begin-comment-gesture',
])

/** All routable kinds — used by tests and any caller that wants full
 *  router authority. Production aboveView passes this, making the router
 *  the canvas-mode authority for selection, drag, resize, marquee, pan,
 *  and edge gestures. */
export const FULL_ROUTER_CONSUME = ALL_KINDS

// Group-label widths for hit geometry, measured with the same font the
// canvas label painter uses so the routable target matches the drawn text.
// Cached per label string; hit-test falls back to an estimate without them.
let labelMeasureCtx: CanvasRenderingContext2D | null = null
const labelWidthCache = new Map<string, number>()

function measureGroupLabelWidth(label: string): number | null {
  const cached = labelWidthCache.get(label)
  if (cached !== undefined) return cached
  labelMeasureCtx ??= document.createElement('canvas').getContext('2d')
  if (!labelMeasureCtx) return null
  labelMeasureCtx.font = GROUP_LABEL_FONT
  const width = labelMeasureCtx.measureText(label).width
  labelWidthCache.set(label, width)
  return width
}

function groupLabelWidths(entities: HitInputs['entities']): Map<string, number> {
  const widths = new Map<string, number>()
  for (const entity of entities) {
    if (entity.kind !== 'group' || !entity.label) continue
    const width = measureGroupLabelWidth(entity.label)
    if (width !== null) widths.set(entity.id, width)
  }
  return widths
}

function layoutToHitInputs(layout: {
  entities: HitInputs['entities']
  edges?: HitInputs['edges'] | null
  selectedEntityIds: HitInputs['selectedEntityIds']
  selectionOperandIds?: HitInputs['selectionOperandIds']
  selectedGroupId?: string | null
  zoom?: number | null
}): HitInputs {
  return {
    entities: layout.entities,
    edges: layout.edges ?? [],
    selectedEntityIds: layout.selectedEntityIds,
    selectionOperandIds: layout.selectionOperandIds,
    selectedGroupId: layout.selectedGroupId ?? null,
    hoveredEntityId: runtimeStore.read().slices.hover?.id ?? null,
    zoom: layout.zoom ?? 1,
    groupLabelWidths: groupLabelWidths(layout.entities),
  }
}

export function useCanvasPointerRouter(options: UseCanvasPointerRouterOptions): void {
  const {
    api,
    layoutRef,
    owner,
    consume,
    spaceHeldRef,
    handToolActiveRef,
    optionHeldRef,
    commandHeldRef,
    setDragCopyPreview,
    setGroupDropTarget,
    setDropBindingSuppressed,
    setEdgeDragState,
    setReorderGhost,
    onCommentDragMove,
    onCommentDragEnd,
    commentDraftRef,
    enteredEntityIdRef,
    onEnterEntityInteractive,
  } = options
  const apiRef = useRef(api)
  apiRef.current = api
  const consumeRef = useRef(consume)
  consumeRef.current = consume
  const setEdgeDragStateRef = useRef(setEdgeDragState)
  setEdgeDragStateRef.current = setEdgeDragState
  const commentGestureRef = useRef({ onCommentDragMove, onCommentDragEnd })
  commentGestureRef.current = { onCommentDragMove, onCommentDragEnd }
  const onEnterEntityInteractiveRef = useRef(onEnterEntityInteractive)
  onEnterEntityInteractiveRef.current = onEnterEntityInteractive

  useEffect(() => {
    if (owner !== 'router' && owner !== 'tool-gesture') return
    const toolGestureOwns = owner === 'tool-gesture'

    // Active placement / comment tool: every canvas pointerdown belongs to
    // the tool. No hit-target routing, no typing-target yield (a comment
    // click on a sticky's textarea anchors a comment, it doesn't focus the
    // note), no edit-commit side effects — only overlay UI wins (I8').
    const handleToolGesturePointerDown = (event: PointerEvent) => {
      if (event.button !== 0) return
      const layout = layoutRef.current
      if (!layout.pendingPlacement && layout.activeTool.kind !== 'comment') return

      const windowY = clientYToWindowY(event.clientY, layout)
      const target = hitTest(layoutToHitInputs(layout), { x: event.clientX, y: windowY })
      const context: CanvasPointerContext = {
        selectedEntityIds: layout.selectedEntityIds,
        selectedGroupId: layout.selectedGroupId ?? null,
        isPrimaryButton: true,
        button: 'left',
        modifiers: { shift: event.shiftKey, meta: event.metaKey, ctrl: event.ctrlKey },
        spaceHeld: spaceHeldRef.current || handToolActiveRef.current,
        altHeld: event.altKey || optionHeldRef.current,
        editingEntityId:
          layout.interaction.kind === 'editing-entity' ? layout.interaction.entityId : null,
        interactivePageId: layout.interactivePageId ?? null,
        interactiveEntityId: enteredEntityIdRef.current,
        placement: layout.pendingPlacement
          ? { entityKind: layout.pendingPlacement.entityKind }
          : null,
        commentToolActive: layout.activeTool.kind === 'comment',
      }
      const action = routePointerDown(target, context)
      const dispatched = dispatchAction({
        action,
        api: apiRef.current,
        event,
        layoutRef,
        optionHeldRef,
        commandHeldRef,
        setDragCopyPreview,
        setGroupDropTarget,
        setDropBindingSuppressed,
        setEdgeDragState: setEdgeDragStateRef.current,
        setReorderGhost,
        onCommentDragMove: commentGestureRef.current.onCommentDragMove,
        onCommentDragEnd: commentGestureRef.current.onCommentDragEnd,
        commentDraftRef,
        onEnterEntityInteractive: onEnterEntityInteractiveRef.current,
      })
      if (dispatched) {
        event.preventDefault()
        event.stopPropagation()
      }
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (isOverlayUiTarget(event.target)) return
      if (toolGestureOwns) return handleToolGesturePointerDown(event)
      handleRouterPointerDown(event, {
        api: apiRef.current,
        layoutRef,
        optionHeldRef,
        commandHeldRef,
        setDragCopyPreview,
        setGroupDropTarget,
        setDropBindingSuppressed,
        setEdgeDragState: setEdgeDragStateRef.current,
        setReorderGhost,
        onCommentDragMove: commentGestureRef.current.onCommentDragMove,
        onCommentDragEnd: commentGestureRef.current.onCommentDragEnd,
        commentDraftRef,
        onEnterEntityInteractive: onEnterEntityInteractiveRef.current,
        consume: consumeRef.current,
        spaceHeld: spaceHeldRef.current,
        handToolActive: handToolActiveRef.current,
        enteredEntityId: enteredEntityIdRef.current,
      })
    }

    const handleDblClick = (event: MouseEvent) => {
      if (isOverlayUiTarget(event.target)) return
      if (isTypingTarget(event.target)) return
      if (event.button !== 0) return
      const layout = layoutRef.current
      const windowY = clientYToWindowY(event.clientY, layout)
      const target = hitTest(layoutToHitInputs(layout), { x: event.clientX, y: windowY })
      const action = routePointerDoubleClick(target)
      switch (action.kind) {
        case 'noop':
          return
        case 'request-entity-edit':
          apiRef.current.requestEntityEdit(action.entityId)
          break
        case 'enter-page-interactive':
          apiRef.current.enterPageInteractive(action.entityId)
          break
        case 'enter-entity-interactive':
          onEnterEntityInteractiveRef.current(action.entityId)
          break
        case 'enter-group':
          apiRef.current.enterGroup(action.groupId)
          break
      }
      event.preventDefault()
      event.stopPropagation()
    }

    const handleContextMenu = (event: MouseEvent) => {
      if (event.defaultPrevented || isTypingTarget(event.target)) return

      const edgeId =
        event.target instanceof Element
          ? event.target.closest<Element>('[data-edge-id]')?.getAttribute('data-edge-id')
          : null
      if (edgeId) {
        apiRef.current.showCanvasItemContextMenu(edgeId)
        event.preventDefault()
        event.stopPropagation()
        return
      }
      if (isOverlayUiTarget(event.target)) return

      const layout = layoutRef.current
      const target = hitTest(
        layoutToHitInputs(layout),
        { x: event.clientX, y: clientYToWindowY(event.clientY, layout) },
      )
      if (target.payload.kind === 'page-body') {
        apiRef.current.showPageContextMenu(target.payload.entityId)
      } else if (
        target.payload.kind === 'entity-body' &&
        // Files retain their richer DOM context menu (Finder/copy/refresh).
        target.payload.entityKind !== 'file'
      ) {
        apiRef.current.showCanvasItemContextMenu(target.payload.entityId)
      } else {
        return
      }
      event.preventDefault()
      event.stopPropagation()
    }

    window.addEventListener('pointerdown', handlePointerDown, { capture: true })
    // Dblclick routing (edit / enter-group / enter-page) is router-mode
    // only — a double click while a tool gesture owns pointers is just two
    // tool gestures.
    if (!toolGestureOwns) {
      window.addEventListener('dblclick', handleDblClick, { capture: true })
      window.addEventListener('contextmenu', handleContextMenu)
    }
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown, {
        capture: true,
      } as EventListenerOptions)
      window.removeEventListener('dblclick', handleDblClick, {
        capture: true,
      } as EventListenerOptions)
      window.removeEventListener('contextmenu', handleContextMenu)
    }
  }, [owner, commandHeldRef, commentDraftRef, handToolActiveRef, layoutRef, optionHeldRef, setDragCopyPreview, setDropBindingSuppressed, setGroupDropTarget, setReorderGhost, spaceHeldRef])
}

function handleRouterPointerDown(event: PointerEvent, deps: RouterPointerDependencies): void {
  if (isTypingTarget(event.target) || !isCanvasPointerButton(event.button)) return
  const layout = deps.layoutRef.current
  const target = hitTest(layoutToHitInputs(layout), {
    x: event.clientX,
    y: clientYToWindowY(event.clientY, layout),
  })
  const editingEntityId = layout.interaction.kind === 'editing-entity'
    ? layout.interaction.entityId
    : null
  commitOutsideInlineEdit(event, target.payload, editingEntityId, deps.api)
  const context = canvasPointerContext(event, layout, editingEntityId, deps)
  const action: CanvasPointerAction = deps.handToolActive && event.button === 0
    ? { kind: 'begin-pan' }
    : routePointerDown(target, context)
  if (!deps.consume.has(action.kind)) return
  if (!dispatchAction({ ...deps, action, event })) return
  event.preventDefault()
  event.stopPropagation()
}

function isCanvasPointerButton(button: number): boolean {
  return button === 0 || button === 1 || button === 2
}

function commitOutsideInlineEdit(
  event: PointerEvent,
  payload: ReturnType<typeof hitTest>['payload'],
  editingEntityId: string | null,
  api: CanvasBgElectronAPI,
): void {
  if (editingEntityId === null || event.button !== 0) return
  const entityId = 'entityId' in payload ? payload.entityId : null
  if (entityId === editingEntityId) return
  commitInlineEditBeforePointerAction(() => {
    const activeElement = document.activeElement
    if (activeElement instanceof HTMLElement) activeElement.blur()
  }, api.commitEntityEdit)
}

function canvasPointerContext(
  event: PointerEvent,
  layout: ProjectedLayoutData,
  editingEntityId: string | null,
  deps: RouterPointerDependencies,
): CanvasPointerContext {
  const modifiers: SelectionModifiers = {
    shift: event.shiftKey,
    meta: event.metaKey,
    ctrl: event.ctrlKey,
  }
  return {
    selectedEntityIds: layout.selectedEntityIds,
    selectedGroupId: layout.selectedGroupId ?? null,
    isPrimaryButton: event.button === 0,
    button: event.button === 1 ? 'middle' : event.button === 2 ? 'right' : 'left',
    modifiers,
    spaceHeld: deps.spaceHeld || deps.handToolActive,
    altHeld: event.altKey || deps.optionHeldRef.current,
    editingEntityId,
    interactivePageId: layout.interactivePageId ?? null,
    interactiveEntityId: deps.enteredEntityId,
    placement: null,
    commentToolActive: false,
  }
}

// --- Dispatch ---

interface DispatchContext extends PointerDispatchDependencies {
  action: CanvasPointerAction
  event: PointerEvent
}

function dispatchAction(ctx: DispatchContext): boolean {
  const { action, api, event, layoutRef, optionHeldRef, setDragCopyPreview, setGroupDropTarget, setEdgeDragState, setReorderGhost } = ctx
  switch (action.kind) {
    case 'noop':
      return false
    case 'page-body-press':
      return runPageBodyPress(action, event, ctx)
    case 'enter-page-interactive':
      api.enterPageInteractive(action.entityId)
      return true
    case 'enter-entity-interactive':
      ctx.onEnterEntityInteractive(action.entityId)
      return true
    case 'forward-pointer-down':
      return runForwardPointer(action, api, event, layoutRef)
    case 'toggle-select':
      if (action.entityKind === 'page') {
        api.selectPage(action.entityId, { shift: true, meta: false, ctrl: false })
      } else {
        api.selectEntity(action.entityId, action.entityKind, {
          shift: true,
          meta: false,
          ctrl: false,
        })
      }
      return true
    case 'group-background-press':
      return runBackgroundSelectionGesture(api, event, layoutRef, {
        entityId: action.groupId,
        entityKind: 'group',
      })
    case 'background-click':
      return runBackgroundSelectionGesture(api, event, layoutRef)
    case 'begin-entity-drag':
      return runEntityDrag(action, event, ctx)
    case 'begin-entity-press':
      return runEntityPress(action, event, ctx)
    case 'begin-group-drag':
      return runGroupDrag(action, event, ctx)
    case 'begin-resize':
      return runResize(action, api, event, layoutRef)
    case 'begin-multi-resize':
      return runMultiResize(action, api, event, layoutRef)
    case 'begin-edge-drag':
      return runEdgeDrag(action, api, event, layoutRef, setEdgeDragState)
    case 'begin-marquee':
      return runBackgroundSelectionGesture(api, event, layoutRef, action.originEntity)
    case 'begin-pan':
      return runPan(api, event)
    case 'begin-reorder-drag':
      return runReorderDrag(action, api, event, layoutRef, setReorderGhost)
    case 'begin-gap-drag':
      return runGapDrag(action, api, event, layoutRef)
    case 'begin-placement':
      return runPlacementGesture(action, api, event, layoutRef)
    case 'begin-comment-gesture':
      return runCommentGesture(
        api,
        event,
        layoutRef,
        ctx.onCommentDragMove,
        ctx.onCommentDragEnd,
        ctx.commentDraftRef,
      )
  }
}

// --- Per-action handlers ---

function runEntityDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-entity-drag' }>,
  event: PointerEvent,
  ctx: PointerDispatchDependencies,
): boolean {
  const releasePointer = capturePointer(event)
  startOptionAwareEntityDrag({
    api: ctx.api,
    layout: ctx.layoutRef.current,
    entityId: action.entityId,
    entityKind: action.entityKind,
    preserveSelection: action.preserveSelection,
    event,
    releasePointer,
    isOptionHeld: () => ctx.optionHeldRef.current,
    isCommandHeld: () => ctx.commandHeldRef.current,
    setPreview: ctx.setDragCopyPreview,
    setGroupDropTarget: ctx.setGroupDropTarget,
    setDropBindingSuppressed: ctx.setDropBindingSuppressed,
  })
  return true
}

function runPromotablePress(input: {
  event: PointerEvent
  promoteToDrag: (pointer: PointerEvent, releasePointer: (() => void) | null) => void
  commitPress: (pointer: PointerEvent) => void
  endDrag: () => void
  ignoreBlurWhileArmed?: boolean
}): boolean {
  let press: PressGestureState = beginPressGesture(input.event.screenX, input.event.screenY)
  const finishIfDragging = (type: 'up' | 'cancel') => {
    if (pressGestureStep(press, { type }).outcome !== 'end-drag') return false
    input.endDrag()
    return true
  }
  const session = startPointerSession(input.event, {
    onMove: (pointer) => {
      const step = pressGestureStep(press, {
        type: 'move',
        x: pointer.screenX,
        y: pointer.screenY,
      })
      press = step.state
      if (step.outcome !== 'promote-to-drag') return
      session.end()
      input.promoteToDrag(pointer, session.releasePointer)
    },
    onUp: (pointer) => {
      if (!finishIfDragging('up')) input.commitPress(pointer)
    },
    onCancel: () => {
      finishIfDragging('cancel')
    },
    listenBlur: true,
    ...(input.ignoreBlurWhileArmed
      ? { ignoreBlur: () => pressGestureIgnoresBlur(press) }
      : {}),
  })
  return true
}

function runEntityPress(
  action: Extract<CanvasPointerAction, { kind: 'begin-entity-press' }>,
  event: PointerEvent,
  ctx: PointerDispatchDependencies,
): boolean {
  return runPromotablePress({
    event,
    promoteToDrag: (initialPointer, releasePointer) => {
      startOptionAwareEntityDrag({
        api: ctx.api,
        layout: ctx.layoutRef.current,
        entityId: action.entityId,
        entityKind: action.entityKind,
        preserveSelection: true,
        event,
        releasePointer,
        initialPointer,
        isOptionHeld: () => ctx.optionHeldRef.current,
        isCommandHeld: () => ctx.commandHeldRef.current,
        setPreview: ctx.setDragCopyPreview,
        setGroupDropTarget: ctx.setGroupDropTarget,
        setDropBindingSuppressed: ctx.setDropBindingSuppressed,
      })
    },
    commitPress: () => ctx.api.requestEntityEdit(action.entityId),
    endDrag: ctx.api.endDragEntity,
    ignoreBlurWhileArmed: true,
  })
}

function runPageBodyPress(
  action: Extract<CanvasPointerAction, { kind: 'page-body-press' }>,
  event: PointerEvent,
  ctx: PointerDispatchDependencies,
): boolean {
  return runPromotablePress({
    event,
    promoteToDrag: (initialPointer, releasePointer) => {
      startOptionAwareEntityDrag({
        api: ctx.api,
        layout: ctx.layoutRef.current,
        entityId: action.entityId,
        entityKind: 'page',
        preserveSelection: action.preserveSelection,
        event,
        releasePointer,
        initialPointer,
        isOptionHeld: () => ctx.optionHeldRef.current,
        isCommandHeld: () => ctx.commandHeldRef.current,
        setPreview: ctx.setDragCopyPreview,
        setGroupDropTarget: ctx.setGroupDropTarget,
        setDropBindingSuppressed: ctx.setDropBindingSuppressed,
      })
    },
    commitPress: (pointer) => {
      // Thread modifiers through so a shift/cmd-click on an unselected or
      // multi-selected page body extends the selection instead of replacing
      // it. Routing already converts additive clicks on page-body to
      // toggle-select, but reading the live modifier state here keeps the
      // gesture honest if the user presses shift between down and up.
      ctx.api.selectPage(action.entityId, {
        shift: pointer.shiftKey,
        meta: pointer.metaKey,
        ctrl: pointer.ctrlKey,
      })
    },
    endDrag: ctx.api.endDragPage,
    ignoreBlurWhileArmed: true,
  })
}

function runGroupDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-group-drag' }>,
  event: PointerEvent,
  ctx: PointerDispatchDependencies,
): boolean {
  return runPromotablePress({
    event,
    promoteToDrag: (initialPointer, releasePointer) => {
      startOptionAwareGroupDrag({
        api: ctx.api,
        layout: ctx.layoutRef.current,
        groupId: action.groupId,
        event,
        releasePointer,
        initialPointer,
        isOptionHeld: () => ctx.optionHeldRef.current,
        isCommandHeld: () => ctx.commandHeldRef.current,
        setPreview: ctx.setDragCopyPreview,
        setGroupDropTarget: ctx.setGroupDropTarget,
        setDropBindingSuppressed: ctx.setDropBindingSuppressed,
      })
    },
    commitPress: () => ctx.api.selectGroup(action.groupId),
    endDrag: ctx.api.endDragEntity,
    // No phantom-blur guard here (§4.6 documents it for entity/page presses
    // only): a window blur cancels a group press even while armed.
  })
}

function runResize(
  action: Extract<CanvasPointerAction, { kind: 'begin-resize' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
): boolean {
  // Capture up front, before the target-entity validation below — a bail
  // leaves the capture held until the implicit release on pointerup.
  capturePointer(event)
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

  // Side handles reflow, corners scale — see `stickyResize.ts`.
  const stickyDispatch = isSticky(entity)
    ? (() => {
        const start = {
          width: entity.width,
          textSize: ('textSize' in entity ? entity.textSize : undefined) ?? TEXT_SIZE_DEFAULT,
        }
        return (patch: { width: number; height: number; canvasX?: number; canvasY?: number }) => {
          api.updateEntity('text', action.entityId, stickyResizePatch(action.handle, start, patch))
        }
      })()
    : null

  // For drawing entities, augment each patch with strokes transformed from the
  // initial bounds so absolute canvas-space stroke geometry tracks the resized
  // selection box in real time.
  const effectiveDispatch = stickyDispatch ?? (entity.kind === 'drawing'
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
          api.updateEntity('drawing', action.entityId, {
            ...patch,
            strokes: scaleStrokesToBounds(initialStrokes, initialBounds, nextBounds),
          })
        }
      })()
    : dispatchPatch)

  // Plain text in 'auto' widthMode is content-driven; the renderer's
  // ResizeObserver overwrites any width/height we'd dispatch. Flip to
  // 'fixed' first so the upcoming width/height patches stick.
  if (entity.kind === 'text' && entity.widthMode === 'auto') {
    api.updateEntity('text', action.entityId, { widthMode: 'fixed' })
  }

  // Enter resize mode in main BEFORE the first dispatchPatch. The bounds-update
  // IPC synchronously requestLayouts; if interactionState is still 'idle' when
  // reconcileFocus runs, focus moves to the selected page (pages only — they
  // populate focusedPageId), aboveView blurs, and the gesture is cancelled
  // after a single tick. Same gotcha as drag-start ordering.
  api.beginResize(action.entityId, entity.kind, action.handle)

  let lastX = event.screenX
  let lastY = event.screenY
  startPointerSession(event, {
    onMove: (ev) => {
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
    },
    onUp: () => api.endResize(),
    onCancel: () => api.endResize(),
    listenBlur: true,
  })
  return true
}

function runMultiResize(
  action: Extract<CanvasPointerAction, { kind: 'begin-multi-resize' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
): boolean {
  // Capture up front, before the selection-bbox validation below — a bail
  // leaves the capture held until the implicit release on pointerup.
  capturePointer(event)
  const layout = layoutRef.current
  // Operand ids (not raw selectedEntityIds) so a group in the selection
  // resizes as its full descendant set, not just its (unrendered) own row —
  // see ADR 0034.
  const seed = computeMultiSelectionBbox(layout.entities, layout.selectionOperandIds)
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
  startPointerSession(event, {
    onMove: (ev) => {
      const screenDx = ev.screenX - lastX
      const screenDy = ev.screenY - lastY
      lastX = ev.screenX
      lastY = ev.screenY
      const entries = applyMultiHandleDelta(acc, action.handle, { screenDx, screenDy, zoom })
      api.resizeMultiSelection(entries)
    },
    onUp: () => api.endMultiResize(),
    onCancel: () => api.endMultiResize(),
    listenBlur: true,
  })
  return true
}

function runEdgeDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-edge-drag' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
  setEdgeDragState: (state: EdgeDragState) => void,
): boolean {
  const layout = layoutRef.current
  const windowY = clientYToWindowY(event.clientY, layout)
  const entityMap = new Map<string, ProjectedSceneEntity>()
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
  const origin = edgeDragOrigin(state)
  if (origin) api.beginEdgeDrag(origin.entityId, origin.side)

  let lastSnap: string | null = null

  const finish = (mode: 'commit' | 'cancel') => {
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

  startPointerSession(event, {
    onMove: (ev) => {
      const cur = layoutRef.current
      const snapMap = new Map<string, ProjectedSceneEntity>()
      for (const e of cur.entities) snapMap.set(e.id, e)
      const winY = clientYToWindowY(ev.clientY, cur)
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
    },
    onUp: () => finish('commit'),
    onCancel: () => finish('cancel'),
    listenBlur: true,
  })
  return true
}

/**
 * Background drag → marquee.
 */
function runBackgroundSelectionGesture(
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
  originEntity?: NonNullable<
    Extract<CanvasPointerAction, { kind: 'begin-marquee' }>['originEntity']
  >,
): boolean {
  const startClientX = event.clientX
  const startClientY = event.clientY
  // Sample the mode modifier live: Cmd/Ctrl can be pressed or released mid-drag
  // to toggle intersect vs. full containment. Accepts any modifier-bearing
  // shape so both pointer and key events feed it.
  const marqueeMode = (m: { metaKey: boolean; ctrlKey: boolean }): MarqueeSelectionMode =>
    m.metaKey || m.ctrlKey ? 'contain' : 'intersect'
  const excludedIds = originEntity ? new Set([originEntity.entityId]) : new Set<string>()
  let dragged = false

  const renderPreview = (clientX: number, clientY: number, mode: MarqueeSelectionMode) => {
    const layout = layoutRef.current
    const rect = normalizeRect(startClientX, startClientY, clientX, clientY)
    const windowRect = {
      left: rect.left,
      top: rect.top + layout.canvasOrigin.y,
      width: rect.width,
      height: rect.height,
    }
    const entityIds = entitiesOverlappingRect(layout.entities, windowRect, {
      mode,
      excludedIds,
    })
    api.setSelectionOverlayRect({
      rect: {
        ...rect,
        top: rect.top + (layout.canvasOrigin.y - TOOLBAR_HEIGHT),
      },
      variant: 'default',
      entityIds,
    })
  }

  startPointerSession(event, {
    onMove: (ev) => {
      if (!dragged) {
        const dx = ev.clientX - startClientX
        const dy = ev.clientY - startClientY
        if (Math.abs(dx) < DRAG_THRESHOLD && Math.abs(dy) < DRAG_THRESHOLD) return
        dragged = true
      }
      renderPreview(ev.clientX, ev.clientY, marqueeMode(ev))
    },
    // Cmd/Ctrl toggled without moving the pointer: re-render at the last
    // cursor position so the preview reflects the new mode immediately.
    onModifiers: (mods, lastPointer) => {
      if (!dragged) return
      renderPreview(lastPointer.clientX, lastPointer.clientY, marqueeMode(mods))
    },
    onUp: (ev) => {
      api.setSelectionOverlayRect(null)
      const layout = layoutRef.current
      // Clicking the dimmed canvas exits focus presentation (camera is otherwise
      // locked; escape and the popup button are the other exits).
      if (!dragged && focusContext(layout).active) {
        api.restoreFocusCamera()
        return
      }
      const modifiers: SelectionModifiers = {
        shift: ev.shiftKey,
        meta: ev.metaKey,
        ctrl: ev.ctrlKey,
      }
      if (!dragged) {
        if (originEntity) {
          if (originEntity.entityKind === 'page') {
            api.selectPage(originEntity.entityId, modifiers)
          } else if (originEntity.entityKind === 'group') {
            api.selectGroup(originEntity.entityId)
          } else {
            api.selectEntity(originEntity.entityId, originEntity.entityKind, modifiers)
          }
          return
        }
        api.canvasDeselect(modifiers)
        return
      }
      const rect = normalizeRect(startClientX, startClientY, ev.clientX, ev.clientY)
      if (rect.width < 4 || rect.height < 4) {
        api.canvasDeselect(modifiers)
        return
      }
      const windowRect = { ...rect, top: rect.top + layout.canvasOrigin.y }
      api.canvasSelectInRect(
        screenRectToCanvasRect(windowRect, layout),
        modifiers,
        {
          selectionMode: marqueeMode(ev),
          excludedEntityIds: [...excludedIds],
        },
      )
    },
    onCancel: () => {
      api.setSelectionOverlayRect(null)
    },
    listenBlur: true,
  })
  return true
}

function runForwardPointer(
  action: Extract<CanvasPointerAction, { kind: 'forward-pointer-down' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
): boolean {
  const { entityId, button } = action
  let lastWindowX = event.clientX
  let lastWindowY = clientYToWindowY(event.clientY, layoutRef.current)
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

  const sendUp = (ev: PointerEvent | null) => {
    const winX = ev ? ev.clientX : lastWindowX
    const winY = ev ? clientYToWindowY(ev.clientY, layoutRef.current) : lastWindowY
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
  // Important: no `listenBlur` here. Forwarding `mouseDown` causes the
  // focus-reconciler to move webContents focus to the target page, which
  // fires `blur` on aboveView. If we treated that as a cancel, we'd tear
  // down the gesture before `pointerup` arrives — leaving the page stuck
  // with a phantom mouseDown and the next click looking like a
  // release+drag rather than a fresh click.
  startPointerSession(event, {
    onMove: (ev) => {
      lastWindowX = ev.clientX
      lastWindowY = clientYToWindowY(ev.clientY, layoutRef.current)
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
    },
    onUp: (ev) => sendUp(ev),
    // Always release the page's mouseDown state so a canceled gesture
    // doesn't leak a stuck button.
    onCancel: () => sendUp(null),
  })
  return true
}

function runPan(api: CanvasBgElectronAPI, event: PointerEvent): boolean {
  let lastScreenX = event.screenX
  let lastScreenY = event.screenY
  startPointerSession(event, {
    onMove: (ev) => {
      const { deltaX, deltaY } = middleDragDelta(
        { screenX: lastScreenX, screenY: lastScreenY },
        ev,
      )
      lastScreenX = ev.screenX
      lastScreenY = ev.screenY
      if (deltaX !== 0 || deltaY !== 0) api.canvasPan(deltaX, deltaY)
    },
    listenBlur: true,
  })
  return true
}

function runReorderDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-reorder-drag' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
  setReorderGhost: (ghost: ReorderGhostOffset) => void,
): boolean {
  // Freeze the grab point so the ghost can float at original-pos + (live -
  // grab) — the grab offset is preserved, keeping the centre dot under the
  // pointer. Canvas-space so it survives pan/zoom mid-drag (it shouldn't, but
  // the math is origin-independent either way).
  const startLayout = layoutRef.current
  const grab = screenPointToCanvasPoint(
    event.clientX,
    clientYToWindowY(event.clientY, startLayout),
    startLayout,
  )

  // Enter reorder mode in main BEFORE any layout-triggering work — same
  // gesture-begin ordering as resize/drag (see runtime/CLAUDE.md). With the
  // mode set to 'reordering-row', the focus reconciler keeps aboveView
  // focused, so the window-blur cancel below doesn't fire on the first tick.
  api.beginReorderDrag(action.movingId)
  // Lift the item the instant it's grabbed (50% ghost in place), before any move.
  setReorderGhost({ dx: 0, dy: 0 })

  startPointerSession(event, {
    onMove: (ev) => {
      const layout = layoutRef.current
      const point = screenPointToCanvasPoint(
        ev.clientX,
        clientYToWindowY(ev.clientY, layout),
        layout,
      )
      api.reorderDragMove(point.x, point.y)
      setReorderGhost({ dx: point.x - grab.x, dy: point.y - grab.y })
    },
    onUp: () => {
      setReorderGhost(null)
      api.reorderDragCommit()
    },
    onCancel: () => {
      setReorderGhost(null)
      api.reorderDragCancel('blur')
    },
    listenBlur: true,
  })
  return true
}

function runGapDrag(
  action: Extract<CanvasPointerAction, { kind: 'begin-gap-drag' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
): boolean {
  const startLayout = layoutRef.current
  const grab = screenPointToCanvasPoint(
    event.clientX,
    clientYToWindowY(event.clientY, startLayout),
    startLayout,
  )

  // Enter gap-resize mode in main BEFORE any layout-triggering work — same
  // gesture-begin ordering as resize/reorder (see runtime/CLAUDE.md). With the
  // mode set to 'resizing-gap', the focus reconciler keeps aboveView focused,
  // so the window-blur cancel below doesn't fire on the first tick. The grab
  // point rides along so main can project moves onto the group's axis.
  api.beginGapResizeDrag(action.groupId, grab.x, grab.y)

  startPointerSession(event, {
    onMove: (ev) => {
      const layout = layoutRef.current
      const point = screenPointToCanvasPoint(
        ev.clientX,
        clientYToWindowY(ev.clientY, layout),
        layout,
      )
      api.gapResizeDragMove(point.x, point.y)
    },
    onUp: () => {
      api.gapResizeDragCommit()
    },
    onCancel: () => {
      api.gapResizeDragCancel('blur')
    },
    listenBlur: true,
  })
  return true
}

const MIN_SHAPE_DRAG_SIZE = 24

function overlayRectFromScreenRect(
  rect: { left: number; top: number; width: number; height: number },
  layout: ProjectedLayoutData,
) {
  return {
    ...rect,
    top: rect.top - layout.canvasOrigin.y,
  }
}

/**
 * Placement-tool gesture. A click places the pending entity at the press
 * point; a shape placement dragged past `MIN_SHAPE_DRAG_SIZE` sizes the shape
 * to the drag rect instead (shift constrains it square), previewed live via
 * the 'place-shape' selection overlay.
 */
function runPlacementGesture(
  action: Extract<CanvasPointerAction, { kind: 'begin-placement' }>,
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
): boolean {
  const layout = layoutRef.current
  const startCanvas = screenPointToCanvasPoint(
    event.clientX,
    clientYToWindowY(event.clientY, layout),
    layout,
  )

  const updateShapePreview = (ev: PointerEvent) => {
    const current = layoutRef.current
    const endCanvas = screenPointToCanvasPoint(
      ev.clientX,
      clientYToWindowY(ev.clientY, current),
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

  startPointerSession(event, {
    onMove: (ev) => {
      if (action.entityKind === 'shape') updateShapePreview(ev)
    },
    onUp: (ev) => {
      if (action.entityKind === 'shape') {
        api.setSelectionOverlayRect(null)
        const current = layoutRef.current
        const endCanvas = screenPointToCanvasPoint(
          ev.clientX,
          clientYToWindowY(ev.clientY, current),
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
    },
    onCancel: () => {
      api.setSelectionOverlayRect(null)
    },
    listenBlur: true,
  })
  return true
}

/**
 * Comment-tool gesture (ADR 0006). Click below threshold → resolve element
 * under cursor via `inspectAtPoint`; element hit → element anchor; nothing →
 * canvas-point anchor. Drag past threshold → marquee → region anchor on
 * pointerup. Threshold matches the rest of the canvas pointer router. Every
 * move/up consults the live tool so leaving comment mode mid-gesture stops
 * it dispatching and clears any marquee overlay it had painted.
 */
function runCommentGesture(
  api: CanvasBgElectronAPI,
  event: PointerEvent,
  layoutRef: LayoutSnapshotRef,
  onDragMove: (startX: number, startY: number, endX: number, endY: number) => void,
  onDragEnd: (startX: number, startY: number, endX: number, endY: number) => void,
  draftRef: React.MutableRefObject<CommentDraftSnapshot>,
): boolean {
  const startX = event.clientX
  const startY = event.clientY
  let crossedThreshold = false

  const session = startPointerSession(event, {
    onMove: (ev) => {
      if (layoutRef.current.activeTool.kind !== 'comment') {
        if (crossedThreshold) {
          api.setSelectionOverlayRect(null)
        }
        session.end()
        return
      }
      if (!crossedThreshold) {
        const dx = ev.clientX - startX
        const dy = ev.clientY - startY
        if (Math.abs(dx) < DRAG_THRESHOLD && Math.abs(dy) < DRAG_THRESHOLD) {
          return
        }
        crossedThreshold = true
      }
      onDragMove(startX, startY, ev.clientX, ev.clientY)
    },
    onUp: (ev) => {
      const current = layoutRef.current
      if (current.activeTool.kind !== 'comment') {
        if (crossedThreshold) {
          api.setSelectionOverlayRect(null)
        }
        return
      }
      if (crossedThreshold) {
        // Drag past threshold → region anchor.
        onDragEnd(startX, startY, ev.clientX, ev.clientY)
        return
      }
      // Click below threshold → element anchor if a page DOM element sits
      // under the cursor (resolved via `inspectAtPoint`), else canvas-point.
      api.setSelectionOverlayRect(null)
      const draft = draftRef.current
      const hasEmptyDraft =
        Boolean(draft.pendingAnnotation || draft.pendingRegionRect) &&
        !draft.commentText.trim()
      if (hasEmptyDraft) {
        // Empty composer open → click-away dismisses it without creating
        // a new draft; comment mode stays active.
        draft.clearDraft()
        return
      }
      api.commitCommentClickAt(ev.clientX, clientYToWindowY(ev.clientY, current))
    },
    onCancel: () => {
      api.setSelectionOverlayRect(null)
    },
    listenBlur: true,
  })
  return true
}

// --- Per-kind helpers ---

function resizeConfigForEntity(entity: ProjectedSceneEntity): ResizeConfig {
  const caps = ENTITY_KIND_CAPS[entity.kind]
  const aspectRatioResizeMode: AspectRatioResizeMode =
    entity.kind === 'file' && 'file' in entity && typeof entity.file === 'string'
      ? aspectRatioResizeModeForCanvasFile(entity.file)
      // A sticky's height is content-driven, so a free vertical drag would do
      // nothing. Locking aspect makes the vertical handles drive width, which
      // is what the text scale follows (`stickyResize.ts`).
      : isSticky(entity)
        ? 'shift-unlocks'
        : caps.aspectMode
  return {
    minWidth: caps.minSize.width,
    minHeight: caps.minSize.height,
    aspectRatioResizeMode,
  }
}

function isSticky(entity: ProjectedSceneEntity): boolean {
  return entity.kind === 'text' && entity.textStyle !== 'plain'
}

function patchDispatcherForKind(
  kind: ProjectedSceneEntity['kind'],
  id: string,
  api: CanvasBgElectronAPI,
): ((patch: { width: number; height: number; canvasX?: number; canvasY?: number }) => void) | null {
  switch (kind) {
    case 'page':
      return (patch) => api.updatePageBounds(id, patch)
    case 'group':
      return (patch) => api.updateEntity('group', id, patch)
    case 'text':
      return (patch) => api.updateEntity('text', id, patch)
    case 'file':
      return (patch) => api.updateEntity('file', id, patch)
    case 'shape':
      return (patch) => api.updateEntity('shape', id, patch)
    case 'drawing':
      return (patch) => api.updateEntity('drawing', id, patch)
    default:
      return null
  }
}
