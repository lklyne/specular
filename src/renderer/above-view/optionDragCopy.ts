import type {
  PointerEvent as ReactPointerEvent,
} from 'react'
import type { CanvasEntityKind, CanvasSceneEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { canvasToScreenX, canvasToScreenY, snapToGrid } from '../../shared/gesture-utils'
import { axisLockDominantAxis, axisLockProjector } from '../../shared/axis-lock-projector'

export type DragCopyPreviewBox = {
  id: string
  left: number
  top: number
  width: number
  height: number
  entityKind: CanvasSceneEntity['kind']
}

type DragPointer = {
  screenX: number
  screenY: number
  altKey?: boolean
  shiftKey?: boolean
}

type DragCopyCallbacks = {
  applyDelta: (dx: number, dy: number, shiftKey: boolean) => void
  previewDelta: (totalDx: number, totalDy: number, shiftKey: boolean) => void
  endDrag: () => void
  copyAt: (canvasX: number, canvasY: number) => void
  setPreview: (preview: DragCopyPreviewBox[]) => void
}

type DragCopySessionOptions = DragCopyCallbacks & {
  layout: LayoutUpdateData
  entityIds: string[]
  anchorEntityId: string
  startScreenX: number
  startScreenY: number
  startShiftKey?: boolean
  isOptionHeld: () => boolean
}

type DragSnapshot = Pick<
  CanvasSceneEntity,
  'id' | 'kind' | 'canvasX' | 'canvasY' | 'screenX' | 'screenY' | 'screenWidth' | 'screenHeight'
>

function draggedEntityIdsForSelection(
  layout: LayoutUpdateData,
  anchorEntityId: string,
): string[] {
  return layout.selectedEntityIds.includes(anchorEntityId)
    ? [...layout.selectedEntityIds]
    : [anchorEntityId]
}

function draggedEntityIdsForGroup(
  layout: LayoutUpdateData,
  groupId: string,
): string[] {
  const ids = new Set<string>([groupId])
  let changed = true
  while (changed) {
    changed = false
    for (const entity of layout.entities) {
      if (entity.kind === 'group' && ids.has(entity.id)) {
        for (const childId of entity.entityIds) {
          if (!ids.has(childId)) {
            ids.add(childId)
            changed = true
          }
        }
      }
      if (
        entity.kind !== 'page' &&
        entity.parentGroupId &&
        ids.has(entity.parentGroupId) &&
        !ids.has(entity.id)
      ) {
        ids.add(entity.id)
        changed = true
      }
    }
  }
  return [...ids]
}

export function createOptionDragCopySession(options: DragCopySessionOptions) {
  const snapshots = options.entityIds
    .map((id) => options.layout.entities.find((entity) => entity.id === id))
    .filter((entity): entity is CanvasSceneEntity => entity !== undefined)
    .map((entity): DragSnapshot => ({
      id: entity.id,
      kind: entity.kind,
      canvasX: entity.canvasX,
      canvasY: entity.canvasY,
      screenX: entity.screenX,
      screenY: entity.screenY,
      screenWidth: entity.screenWidth,
      screenHeight: entity.screenHeight,
    }))
  const anchorSnapshot =
    snapshots.find((entity) => entity.id === options.anchorEntityId) ?? snapshots[0]
  const minCanvasX = snapshots.length
    ? Math.min(...snapshots.map((entity) => entity.canvasX))
    : 0
  const minCanvasY = snapshots.length
    ? Math.min(...snapshots.map((entity) => entity.canvasY))
    : 0
  const anchorCanvasX = anchorSnapshot?.kind === 'group' ? anchorSnapshot.canvasX : minCanvasX
  const anchorCanvasY = anchorSnapshot?.kind === 'group' ? anchorSnapshot.canvasY : minCanvasY

  let lastScreenX = options.startScreenX
  let lastScreenY = options.startScreenY
  let totalScreenDx = 0
  let totalScreenDy = 0
  let appliedScreenDx = 0
  let appliedScreenDy = 0
  let shiftKey = Boolean(options.startShiftKey)
  let copyMode = false
  let hasMoved = false
  let finished = false

  const targetCanvasPoint = () => {
    const rawDelta = {
      x: totalScreenDx / options.layout.zoom,
      y: totalScreenDy / options.layout.zoom,
    }
    const projected = axisLockProjector(rawDelta, rawDelta, shiftKey)
    const dominant = axisLockDominantAxis(rawDelta, shiftKey)
    const projectedX = anchorCanvasX + projected.x
    const projectedY = anchorCanvasY + projected.y
    return {
      canvasX: dominant === 'vertical' ? projectedX : snapToGrid(projectedX),
      canvasY: dominant === 'horizontal' ? projectedY : snapToGrid(projectedY),
    }
  }

  const buildPreview = (): DragCopyPreviewBox[] => {
    const target = targetCanvasPoint()
    return snapshots.map((entity) => {
      const canvasX = target.canvasX + (entity.canvasX - anchorCanvasX)
      const canvasY = target.canvasY + (entity.canvasY - anchorCanvasY)
      return {
        id: entity.id,
        entityKind: entity.kind,
        left: canvasToScreenX(options.layout, canvasX),
        top: canvasToScreenY(options.layout, canvasY) - options.layout.canvasOrigin.y,
        width: entity.screenWidth,
        height: entity.screenHeight,
      }
    })
  }

  const setCopyMode = (nextCopyMode: boolean) => {
    if (finished) return
    copyMode = nextCopyMode
    if (copyMode) {
      if (appliedScreenDx !== 0 || appliedScreenDy !== 0) {
        options.applyDelta(-appliedScreenDx, -appliedScreenDy, shiftKey)
        appliedScreenDx = 0
        appliedScreenDy = 0
      }
      options.setPreview(buildPreview())
      options.previewDelta(totalScreenDx, totalScreenDy, shiftKey)
      return
    }

    options.setPreview([])
    const dx = totalScreenDx - appliedScreenDx
    const dy = totalScreenDy - appliedScreenDy
    if (dx !== 0 || dy !== 0) {
      options.applyDelta(dx, dy, shiftKey)
      appliedScreenDx = totalScreenDx
      appliedScreenDy = totalScreenDy
    }
  }

  setCopyMode(options.isOptionHeld())

  return {
    move(pointer: DragPointer) {
      if (finished) return
      shiftKey = Boolean(pointer.shiftKey)
      const dx = pointer.screenX - lastScreenX
      const dy = pointer.screenY - lastScreenY
      lastScreenX = pointer.screenX
      lastScreenY = pointer.screenY
      if (dx !== 0 || dy !== 0) {
        hasMoved = true
        totalScreenDx += dx
        totalScreenDy += dy
      }
      setCopyMode(Boolean(pointer.altKey) || options.isOptionHeld())
    },
    setShiftKey(held: boolean) {
      if (finished || shiftKey === held) return
      shiftKey = held
      if (copyMode) {
        options.setPreview(buildPreview())
        options.previewDelta(totalScreenDx, totalScreenDy, shiftKey)
      } else {
        options.applyDelta(0, 0, shiftKey)
      }
    },
    setOptionHeld(held: boolean) {
      setCopyMode(held)
    },
    finish(pointer?: DragPointer | null) {
      if (finished) return
      if (pointer) this.move(pointer)
      const shouldCopy = copyMode && hasMoved && snapshots.length > 0
      if (shouldCopy && (appliedScreenDx !== 0 || appliedScreenDy !== 0)) {
        options.applyDelta(-appliedScreenDx, -appliedScreenDy, shiftKey)
        appliedScreenDx = 0
        appliedScreenDy = 0
      }
      finished = true
      options.setPreview([])
      options.endDrag()
      if (shouldCopy) {
        const point = targetCanvasPoint()
        options.copyAt(point.canvasX, point.canvasY)
      }
    },
    cancel() {
      if (finished) return
      if (appliedScreenDx !== 0 || appliedScreenDy !== 0) {
        options.applyDelta(-appliedScreenDx, -appliedScreenDy, shiftKey)
      }
      finished = true
      options.setPreview([])
      options.endDrag()
    },
  }
}

export function startOptionAwareEntityDrag(input: {
  api: CanvasBgElectronAPI
  layout: LayoutUpdateData
  entityId: string
  entityKind: CanvasEntityKind
  preserveSelection: boolean
  event: PointerEvent | ReactPointerEvent
  releasePointer?: (() => void) | null
  captureTarget?: Element | null
  initialPointer?: DragPointer
  isOptionHeld: () => boolean
  setPreview: (preview: DragCopyPreviewBox[]) => void
}) {
  const pointerId = input.event.pointerId
  const entityIds = draggedEntityIdsForSelection(input.layout, input.entityId)
  if (input.entityKind === 'page') {
    input.api.startDragPage(input.entityId, {
      entityKind: 'page',
      preserveSelection: input.preserveSelection,
    })
  } else {
    input.api.startDragEntity(input.entityId, {
      entityKind: input.entityKind,
      preserveSelection: input.preserveSelection,
    })
  }

  const release = () => {
    input.releasePointer?.()
    if (!input.captureTarget) return
    try {
      if (input.captureTarget.hasPointerCapture(pointerId)) {
        input.captureTarget.releasePointerCapture(pointerId)
      }
    } catch {
      /* ignore */
    }
  }

  const session = createOptionDragCopySession({
    layout: input.layout,
    entityIds,
    anchorEntityId: input.entityId,
    startScreenX: input.event.screenX,
    startScreenY: input.event.screenY,
    startShiftKey: input.event.shiftKey,
    isOptionHeld: input.isOptionHeld,
    setPreview: input.setPreview,
    applyDelta: (dx, dy, shiftKey) => {
      if (input.entityKind === 'page') input.api.dragPage(input.entityId, dx, dy, shiftKey)
      else input.api.dragEntity(input.entityId, dx, dy, shiftKey)
    },
    previewDelta: (totalDx, totalDy, shiftKey) => input.api.dragPreview(totalDx, totalDy, shiftKey),
    endDrag: () => {
      release()
      if (input.entityKind === 'page') input.api.endDragPage()
      else input.api.endDragEntity()
    },
    copyAt: (canvasX, canvasY) => input.api.dragCopySelection(canvasX, canvasY),
  })
  if (input.initialPointer) session.move(input.initialPointer)

  return installOptionAwareDragListeners({
    pointerId,
    session,
    isOptionHeld: input.isOptionHeld,
  })
}

export function startOptionAwareGroupDrag(input: {
  api: CanvasBgElectronAPI
  layout: LayoutUpdateData
  groupId: string
  event: PointerEvent | ReactPointerEvent
  releasePointer?: (() => void) | null
  captureTarget?: Element | null
  initialPointer?: DragPointer
  isOptionHeld: () => boolean
  setPreview: (preview: DragCopyPreviewBox[]) => void
}) {
  const pointerId = input.event.pointerId
  const entityIds = draggedEntityIdsForGroup(input.layout, input.groupId)
  input.api.startDragGroup(input.groupId)

  const release = () => {
    input.releasePointer?.()
    if (!input.captureTarget) return
    try {
      if (input.captureTarget.hasPointerCapture(pointerId)) {
        input.captureTarget.releasePointerCapture(pointerId)
      }
    } catch {
      /* ignore */
    }
  }

  const session = createOptionDragCopySession({
    layout: input.layout,
    entityIds,
    anchorEntityId: input.groupId,
    startScreenX: input.event.screenX,
    startScreenY: input.event.screenY,
    startShiftKey: input.event.shiftKey,
    isOptionHeld: input.isOptionHeld,
    setPreview: input.setPreview,
    applyDelta: (dx, dy, shiftKey) => input.api.dragGroup(input.groupId, dx, dy, shiftKey),
    previewDelta: (totalDx, totalDy, shiftKey) => input.api.dragPreview(totalDx, totalDy, shiftKey),
    endDrag: () => {
      release()
      input.api.endDragGroup()
    },
    copyAt: (canvasX, canvasY) => input.api.dragCopyGroup(input.groupId, canvasX, canvasY),
  })
  if (input.initialPointer) session.move(input.initialPointer)

  return installOptionAwareDragListeners({
    pointerId,
    session,
    isOptionHeld: input.isOptionHeld,
  })
}

function installOptionAwareDragListeners(input: {
  pointerId: number
  session: ReturnType<typeof createOptionDragCopySession>
  isOptionHeld: () => boolean
}) {
  const cleanup = () => {
    window.removeEventListener('pointermove', onPointerMove)
    window.removeEventListener('pointerup', onPointerUp)
    window.removeEventListener('pointercancel', onCancel)
    window.removeEventListener('keydown', onKeyChange)
    window.removeEventListener('keyup', onKeyChange)
    window.removeEventListener('blur', onCancel)
  }
  const onPointerMove = (event: PointerEvent) => {
    if (event.pointerId !== input.pointerId) return
    input.session.move(event)
  }
  const onPointerUp = (event: PointerEvent) => {
    if (event.pointerId !== input.pointerId) return
    cleanup()
    input.session.finish(event)
  }
  const onKeyChange = (event: KeyboardEvent) => {
    input.session.setShiftKey(event.shiftKey)
    input.session.setOptionHeld(input.isOptionHeld())
  }
  const onCancel = () => {
    cleanup()
    input.session.cancel()
  }

  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', onPointerUp)
  window.addEventListener('pointercancel', onCancel)
  window.addEventListener('keydown', onKeyChange)
  window.addEventListener('keyup', onKeyChange)
  window.addEventListener('blur', onCancel)

  return cleanup
}
