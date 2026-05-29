/**
 * FileChrome — per-file-entity chrome rendered in aboveView. Per ADR 0008 §6,
 * the chrome is identity-only: favicon + filename label, no actions. All
 * file actions (rename, wireframe theme, json toggle, dup, del) live in the
 * `FilePopup`, dispatched through the renderer plugin contribution surface
 * (ADR 0008 §7).
 */

import { memo, type MutableRefObject } from 'react'
import type {
  CanvasBgElectronAPI,
  CanvasSceneFileEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { MARKDOWN_EXTENSIONS, WIREFRAME_EXTENSIONS } from '../canvas-bg/entityConstants'
import { CanvasItemChrome } from './CanvasItemChrome'
import { iconForFilePath } from '../shared/fileIcon'
import { startOptionAwareEntityDrag, type DragCopyPreviewBox } from './optionDragCopy'

export function FileChromeOverlay({
  api,
  layoutData,
  isDark,
  optionHeldRef,
  setDragCopyPreview,
}: {
  api: CanvasBgElectronAPI
  layoutData: LayoutUpdateData
  isDark: boolean
  optionHeldRef: MutableRefObject<boolean>
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void
}) {
  if (layoutData.viewMode !== 'canvas') return null
  const fileEntities = layoutData.entities.filter(
    (e): e is CanvasSceneFileEntity => e.kind === 'file',
  )
  const isIdle = layoutData.interaction.kind === 'idle'
  const selectedEntityId =
    layoutData.selectedEntityIds.length === 1 ? layoutData.selectedEntityIds[0] : null
  const hoveredEntityId = layoutData.hover?.id ?? null
  return (
    <>
      {fileEntities.map((entity) => {
        // Image entities never show the filename chrome — they render bare, and
        // the selection popover surfaces the name when selected.
        if (entity.rendererTag === 'image') return null
        return (
          <FileChromeItem
            key={entity.id}
            api={api}
            layoutData={layoutData}
            entity={entity}
            isDark={isDark}
            isActive={(entity.id === selectedEntityId && isIdle) || entity.id === hoveredEntityId}
            optionHeldRef={optionHeldRef}
            setDragCopyPreview={setDragCopyPreview}
          />
        )
      })}
    </>
  )
}

const FileChromeItem = memo(function FileChromeItem({
  api,
  layoutData,
  entity,
  isDark,
  isActive,
  optionHeldRef,
  setDragCopyPreview,
}: {
  api: CanvasBgElectronAPI
  layoutData: LayoutUpdateData
  entity: CanvasSceneFileEntity
  isDark: boolean
  isActive: boolean
  optionHeldRef: MutableRefObject<boolean>
  setDragCopyPreview: (preview: DragCopyPreviewBox[]) => void
}) {
  const fileName = entity.file.split('/').pop() ?? entity.file
  const displayName = WIREFRAME_EXTENSIONS.test(entity.file)
    ? fileName.replace(/\.wireframe\.json$/i, '')
    : MARKDOWN_EXTENSIONS.test(entity.file)
      ? fileName.replace(/\.md$/i, '')
      : fileName
  const FileIcon = iconForFilePath(entity.file)

  const onPointerDown = (event: React.PointerEvent) => {
    const target = event.target as HTMLElement
    if (target.closest('[data-page-context-menu]')) return
    event.preventDefault()
    event.stopPropagation()
    const pointerId = event.pointerId
    const captureTarget = event.currentTarget
    try {
      captureTarget.setPointerCapture(pointerId)
    } catch {
      /* ignore */
    }
    const additive = event.shiftKey || event.metaKey || event.ctrlKey
    const modifiers = { shift: event.shiftKey, meta: event.metaKey, ctrl: event.ctrlKey }
    if (additive) {
      api.selectEntity(entity.id, 'file', modifiers)
      try {
        if (captureTarget.hasPointerCapture(pointerId)) {
          captureTarget.releasePointerCapture(pointerId)
        }
      } catch {
        /* ignore */
      }
      return
    }
    const preserve = layoutData.selectedEntityIds.includes(entity.id)
    startOptionAwareEntityDrag({
      api,
      layout: layoutData,
      entityId: entity.id,
      entityKind: 'file',
      preserveSelection: preserve,
      event,
      captureTarget,
      isOptionHeld: () => optionHeldRef.current,
      setPreview: setDragCopyPreview,
    })
  }

  return (
    <CanvasItemChrome.Root
      entityId={entity.id}
      layout={layoutData}
      isDark={isDark}
      isActive={isActive}
      onPointerDown={onPointerDown}
      onPointerEnter={() => api.hoverPage(entity.id)}
      onPointerLeave={() => api.hoverPage(null)}
    >
      <CanvasItemChrome.DragTrigger>
        <FileIcon size={13} className="shrink-0 text-zinc-400" />
        <CanvasItemChrome.Title>{displayName}</CanvasItemChrome.Title>
      </CanvasItemChrome.DragTrigger>
    </CanvasItemChrome.Root>
  )
})
