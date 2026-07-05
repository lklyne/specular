/**
 * GroupRenameLabel — group name + rename trigger rendered in aboveView.
 * Per ADR 0002 §2 the label sits above each group's bounds and is
 * `data-overlay-ui` so the canvas-pointer-router yields. Group bounds
 * (the bordered rect itself) keep rendering in canvas-bg via
 * `GroupBoundsLayer` — only the rename label moves.
 *
 * Edit-mode entry/commit/cancel flows through the unified
 * `canvas-{request,commit,cancel}-entity-edit` IPC pair (the same
 * channel used by sticky/text/shape bodies); `isRenaming` is derived
 * from `editingEntityId === group.id`, never local state.
 */

import { FolderOpen } from 'lucide-react'
import type { MutableRefObject } from 'react'
import type { CanvasSceneGroupEntity, LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { DRAG_THRESHOLD } from '../../shared/gesture-utils'
import { InlineEditLabel } from '../shared/InlineEditLabel'
import { startOptionAwareGroupDrag, type DragCopyPreviewBox } from './optionDragCopy'

export function GroupRenameOverlay({
  api,
  layoutData,
  isDark,
  editingEntityId,
  optionHeldRef,
  setDragCopyPreview,
}: {
  api: CanvasBgElectronAPI
  layoutData: LayoutUpdateData
  isDark: boolean
  editingEntityId: string | null
  optionHeldRef: MutableRefObject<boolean>
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void
}) {
  const groups = layoutData.groups ?? []
  if (!groups.length) return null
  return (
    <>
      {groups.map((group) => (
        <GroupRenameItem
          key={group.id}
          api={api}
          layoutData={layoutData}
          group={group}
          isDark={isDark}
          isRenaming={editingEntityId === group.id}
          optionHeldRef={optionHeldRef}
          setDragCopyPreview={setDragCopyPreview}
        />
      ))}
    </>
  )
}

function GroupRenameItem({
  api,
  layoutData,
  group,
  isDark,
  isRenaming,
  optionHeldRef,
  setDragCopyPreview,
}: {
  api: CanvasBgElectronAPI
  layoutData: LayoutUpdateData
  group: CanvasSceneGroupEntity
  isDark: boolean
  isRenaming: boolean
  optionHeldRef: MutableRefObject<boolean>
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void
}) {
  const labelColorClass = group.color
    ? isDark ? 'text-zinc-100' : 'text-zinc-900'
    : isDark ? 'text-zinc-300' : 'text-zinc-700'
  const iconColorClass = group.color
    ? isDark ? 'text-zinc-300' : 'text-zinc-600'
    : 'text-zinc-500'
  // The label sits above group.screenY and inside aboveView's overlay-local
  // coordinate space; subtract canvasOrigin.y to drop into overlay coords.
  const left = group.screenX
  const top = group.screenY - layoutData.canvasOrigin.y

  const onPointerDown = isRenaming
    ? (event: React.PointerEvent) => event.stopPropagation()
    : (event: React.PointerEvent) => {
        event.preventDefault()
        event.stopPropagation()
        const additive = event.shiftKey || event.metaKey || event.ctrlKey
        if (additive) {
          api.selectGroup(group.id)
          return
        }
        const pointerId = event.pointerId
        let dragging = false
        const startX = event.screenX
        const startY = event.screenY
        const onMove = (ev: PointerEvent) => {
          if (ev.pointerId !== pointerId) return
          const totalDx = ev.screenX - startX
          const totalDy = ev.screenY - startY
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
              layout: layoutData,
              groupId: group.id,
              event: event.nativeEvent,
              initialPointer: ev,
              isOptionHeld: () => optionHeldRef.current,
              setPreview: setDragCopyPreview,
            })
            return
          }
        }
        const cleanup = () => {
          window.removeEventListener('pointermove', onMove)
          window.removeEventListener('pointerup', onUp)
          window.removeEventListener('blur', onCancel)
        }
        const onUp = (ev: PointerEvent) => {
          if (ev.pointerId !== pointerId) return
          cleanup()
          if (dragging) {
            api.endDragGroup()
            return
          }
          api.selectGroup(group.id)
        }
        const onCancel = () => {
          cleanup()
          if (dragging) api.endDragGroup()
        }
        window.addEventListener('pointermove', onMove)
        window.addEventListener('pointerup', onUp)
        window.addEventListener('blur', onCancel)
      }

  return (
    <div
      data-overlay-ui
      className={`pointer-events-auto absolute select-none text-[11px] font-medium ${labelColorClass}`}
      style={{
        left,
        top,
        transform: 'translateY(-100%)',
        whiteSpace: 'nowrap',
        cursor: isRenaming ? 'text' : 'grab',
      }}
      onPointerDown={onPointerDown}
      onDoubleClick={() => api.requestEntityEdit(group.id)}
    >
      <span className="inline-flex items-center gap-1 pb-1">
        <FolderOpen size={14} className={`shrink-0 ${iconColorClass}`} />
        <InlineEditLabel
          value={group.label}
          isEditing={isRenaming}
          onStartEdit={() => api.requestEntityEdit(group.id)}
          onCommit={(next) => {
            api.renameGroup(group.id, next)
            api.commitEntityEdit()
          }}
          onCancel={() => api.cancelEntityEdit()}
          variant="canvas-chrome"
          isDark={isDark}
          titleClassName="min-w-0 truncate"
          inputClassName="min-w-[120px] border-0 bg-transparent text-[11px] font-medium outline-none focus:outline-none"
        />
      </span>
    </div>
  )
}
