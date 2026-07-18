import type { CanvasInteractionState, CanvasSceneEntity, LayoutUpdateData } from './types'
import type { InteractionMode } from './interaction-types'
import { GRID_SIZE } from './constants'
import type { MarqueeSelectionMode } from './marquee-selection'

export const DRAG_THRESHOLD = 4

export {
  canvasToScreenX,
  canvasToScreenY,
  clientYToWindowY,
  screenPointToCanvasPoint,
  screenRectToCanvasRect,
  toOverlayY,
} from './coords'

type ViewportWheelAction =
  | {
      kind: 'zoom'
      deltaY: number
      mouseX: number
      mouseY: number
    }
  | {
      kind: 'pan'
      deltaX: number
      deltaY: number
    }

export function normalizeRect(
  startX: number,
  startY: number,
  currentX: number,
  currentY: number,
) {
  const left = Math.min(startX, currentX)
  const top = Math.min(startY, currentY)
  return {
    left,
    top,
    width: Math.abs(currentX - startX),
    height: Math.abs(currentY - startY),
  }
}

export function squareConstrainedRect(
  startX: number,
  startY: number,
  currentX: number,
  currentY: number,
  constrainSquare: boolean,
) {
  if (!constrainSquare) return normalizeRect(startX, startY, currentX, currentY)
  const dx = currentX - startX
  const dy = currentY - startY
  const side = Math.max(Math.abs(dx), Math.abs(dy))
  return {
    left: dx < 0 ? startX - side : startX,
    top: dy < 0 ? startY - side : startY,
    width: side,
    height: side,
  }
}

export function snapToGrid(value: number): number {
  return Math.round(value / GRID_SIZE) * GRID_SIZE
}

export type GeometryPatchKey = 'canvasX' | 'canvasY' | 'width' | 'height'

const ALL_GEOMETRY_PATCH_KEYS: readonly GeometryPatchKey[] = [
  'canvasX',
  'canvasY',
  'width',
  'height',
]

/**
 * Grid-snap the geometry fields present on a partial entity patch, leaving
 * every other field untouched. `keys` narrows which fields snap — file
 * entities snap position but keep their intrinsic size.
 */
export function snapGeometryPatch<T extends Partial<Record<GeometryPatchKey, number>>>(
  patch: T,
  keys: readonly GeometryPatchKey[] = ALL_GEOMETRY_PATCH_KEYS,
): T {
  const snapped = { ...patch }
  for (const key of keys) {
    const value = snapped[key]
    if (value !== undefined) snapped[key] = snapToGrid(value) as T[GeometryPatchKey]
  }
  return snapped
}

export function isTypingTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLInputElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  )
}

export function isOverlayUiTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  // Resize handles live inside the selection overlay (which is tagged
  // `data-overlay-ui`) but they ARE routable — the canvas pointer router
  // hit-tests the click position to dispatch begin-resize.
  if (target.closest('[data-resize-handle]')) return false
  return Boolean(target.closest('[data-overlay-ui]'))
}

// Overlay UI that captures clicks but must NOT swallow canvas pan/zoom — e.g.
// the region-annotation box: clickable to open its thread, yet the user still
// expects wheel/middle-drag to pan and zoom the canvas underneath it. Stays
// `isOverlayUiTarget` for the pointer router (no marquee on click); opts out
// here so the viewport gesture hooks let the gesture through.
export function blocksViewportGesture(target: EventTarget | null): boolean {
  if (target instanceof Element && target.closest('[data-viewport-passthrough]')) return false
  return isOverlayUiTarget(target)
}

type ScrollAxis = 'x' | 'y' | 'either'

function isScrollableContainer(el: HTMLElement, axis: ScrollAxis): boolean {
  // Cheap overflow check first — only resolve computed style when the content
  // actually overflows the box on a relevant axis (getComputedStyle can force
  // a style flush, and this runs per ancestor per wheel tick).
  const overflowsY = el.scrollHeight > el.clientHeight
  const overflowsX = el.scrollWidth > el.clientWidth
  if (axis === 'y' && !overflowsY) return false
  if (axis === 'x' && !overflowsX) return false
  if (axis === 'either' && !overflowsY && !overflowsX) return false

  const style = getComputedStyle(el)
  const scrollableY =
    overflowsY && (style.overflowY === 'auto' || style.overflowY === 'scroll')
  const scrollableX =
    overflowsX && (style.overflowX === 'auto' || style.overflowX === 'scroll')
  if (axis === 'y') return scrollableY
  if (axis === 'x') return scrollableX
  return scrollableY || scrollableX
}

/**
 * True when a wheel event's target sits inside a scrollable element belonging
 * to an entity body (a markdown note, etc.). The walk stops at the entity
 * shell (`[data-entity-id]`) so it never reports the canvas surface itself.
 *
 * Used so wheeling over a note's body scrolls the note natively instead of
 * the canvas-mode wheel authority swallowing the event to pan the canvas.
 *
 * When `wheel` is supplied, only the wheel's dominant axis counts as
 * scrollable — so a vertical wheel over a body that only overflows
 * horizontally is NOT yielded (native scroll couldn't move it, which would
 * otherwise deaden the wheel).
 */
export function canScrollWheelTarget(
  target: EventTarget | null,
  wheel?: Pick<WheelEvent, 'deltaX' | 'deltaY'>,
): boolean {
  const axis: ScrollAxis = wheel
    ? Math.abs(wheel.deltaY) >= Math.abs(wheel.deltaX)
      ? 'y'
      : 'x'
    : 'either'
  let node: Element | null = target instanceof Element ? target : null
  while (node) {
    if (node instanceof HTMLElement && isScrollableContainer(node, axis)) return true
    if (node.hasAttribute('data-entity-id')) break
    node = node.parentElement
  }
  return false
}

function hasNoModifierKeys(
  event: Pick<KeyboardEvent, 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>,
): boolean {
  return !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey
}

export function isPlainShortcutKey(
  event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>,
  key: string,
): boolean {
  return event.key.toLowerCase() === key.toLowerCase() && hasNoModifierKeys(event)
}

export function canvasInteractionModeKind(state: CanvasInteractionState): InteractionMode['kind'] {
  switch (state.kind) {
    case 'idle': return 'idle'
    case 'panning-canvas': return 'panning'
    case 'marquee-select': return 'marquee'
    case 'dragging-entities': return 'dragging-entities'
    case 'resizing-entity': return 'resizing-entity'
    case 'resizing-multi-selection': return 'resizing-multi-selection'
    case 'dragging-edge': return 'dragging-edge'
    case 'editing-entity': return 'editing-entity'
    case 'reordering-row': return 'reordering-row'
    case 'resizing-gap': return 'resizing-gap'
  }
}

export function classifyViewportWheel(event: Pick<WheelEvent, 'metaKey' | 'ctrlKey' | 'deltaX' | 'deltaY' | 'screenX' | 'screenY'>): ViewportWheelAction {
  if (event.metaKey || event.ctrlKey) {
    return {
      kind: 'zoom',
      deltaY: event.deltaY,
      mouseX: event.screenX,
      mouseY: event.screenY,
    }
  }
  return {
    kind: 'pan',
    deltaX: event.deltaX,
    deltaY: event.deltaY,
  }
}

export function shouldStartMouseViewportPan(event: Pick<PointerEvent, 'button'>): boolean {
  return event.button === 1
}

export function middleDragDelta(
  previous: { screenX: number; screenY: number },
  next: Pick<PointerEvent, 'screenX' | 'screenY'>,
) {
  return {
    deltaX: previous.screenX - next.screenX,
    deltaY: previous.screenY - next.screenY,
  }
}

export interface MarqueeSelectionCandidate {
  id: string
  kind: CanvasSceneEntity['kind']
  parentGroupId?: string
  left: number
  top: number
  width: number
  height: number
}

export interface MarqueeSelectionRect {
  left: number
  top: number
  width: number
  height: number
}

function rectsOverlap(
  candidate: MarqueeSelectionCandidate,
  rect: MarqueeSelectionRect,
): boolean {
  const right = rect.left + rect.width
  const bottom = rect.top + rect.height
  return (
    rect.left < candidate.left + candidate.width &&
    right > candidate.left &&
    rect.top < candidate.top + candidate.height &&
    bottom > candidate.top
  )
}

function rectContains(
  rect: MarqueeSelectionRect,
  candidate: MarqueeSelectionCandidate,
): boolean {
  return (
    rect.left <= candidate.left &&
    rect.top <= candidate.top &&
    rect.left + rect.width >= candidate.left + candidate.width &&
    rect.top + rect.height >= candidate.top + candidate.height
  )
}

export interface MarqueeSelectionOptions {
  /** Leaf hit test: 'intersect' selects overlapping items, 'contain' only fully enclosed ones (Cmd/Ctrl marquee). */
  mode?: MarqueeSelectionMode
  /** Ids excluded from the result — e.g. the item under the marquee's origin click. */
  excludedIds?: ReadonlySet<string>
}

/**
 * Resolve marquee targets with group-aware containment:
 *
 * - a group is selectable only when the marquee fully encloses its bounds;
 * - a partially crossed group is transparent, exposing overlapping children;
 * - fully enclosed nested groups collapse to their outer fully enclosed group;
 * - independent full groups and loose hits from partial groups batch together.
 *
 * Group enclosure always uses full containment (a group is a unit only when
 * fully covered); `mode` only governs the leaf hit test, and `excludedIds`
 * drops the origin item so a Cmd-marquee can begin through a body.
 */
export function resolveMarqueeSelectionIds(
  candidates: readonly MarqueeSelectionCandidate[],
  rect: MarqueeSelectionRect,
  { mode = 'intersect', excludedIds }: MarqueeSelectionOptions = {},
): string[] {
  const candidateById = new Map(candidates.map((candidate) => [candidate.id, candidate]))
  const fullyEnclosedGroupIds = new Set(
    candidates
      .filter((candidate) => candidate.kind === 'group' && rectContains(rect, candidate))
      .map((candidate) => candidate.id),
  )
  const outerFullyEnclosedGroups = candidates.filter((candidate) => {
    if (!fullyEnclosedGroupIds.has(candidate.id)) return false
    let parentGroupId = candidate.parentGroupId
    while (parentGroupId) {
      if (fullyEnclosedGroupIds.has(parentGroupId)) return false
      parentGroupId = candidateById.get(parentGroupId)?.parentGroupId
    }
    return true
  })
  const selectedGroupIds = new Set(outerFullyEnclosedGroups.map((group) => group.id))
  const isInsideSelectedGroup = (candidate: MarqueeSelectionCandidate): boolean => {
    let parentGroupId = candidate.parentGroupId
    while (parentGroupId) {
      if (selectedGroupIds.has(parentGroupId)) return true
      parentGroupId = candidateById.get(parentGroupId)?.parentGroupId
    }
    return false
  }

  return candidates
    .filter((candidate) => {
      if (excludedIds?.has(candidate.id)) return false
      if (selectedGroupIds.has(candidate.id)) return true
      if (candidate.kind === 'group' || isInsideSelectedGroup(candidate)) return false
      return mode === 'contain'
        ? rectContains(rect, candidate)
        : rectsOverlap(candidate, rect)
    })
    .map((candidate) => candidate.id)
}

/**
 * Scene-entity adapter used by the renderer's live marquee preview.
 */
export function entitiesOverlappingRect(
  entities: readonly CanvasSceneEntity[],
  rect: MarqueeSelectionRect,
  options?: MarqueeSelectionOptions,
): string[] {
  const parentGroupByChildId = new Map<string, string>()
  for (const entity of entities) {
    if (entity.kind !== 'group') continue
    for (const childId of entity.entityIds) {
      parentGroupByChildId.set(childId, entity.id)
    }
  }
  return resolveMarqueeSelectionIds(
    entities.map((entity) => ({
      id: entity.id,
      kind: entity.kind,
      parentGroupId:
        'parentGroupId' in entity
          ? entity.parentGroupId
          : parentGroupByChildId.get(entity.id),
      left: entity.screenX,
      top: entity.screenY,
      width: entity.screenWidth,
      height: entity.screenHeight,
    })),
    rect,
    options,
  )
}
