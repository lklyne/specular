/**
 * FileBodyLayer — file-entity bodies (image, video, markdown,
 * component placeholder, fallback). Mounted in aboveView so a file placed
 * over a page is actually drawn above it.
 *
 * Hit-tests run in `useCanvasPointerRouter` against the layout snapshot
 * (front-to-back), so this layer is purely visual for selection/drag/resize.
 * The contenteditable inside the markdown renderer is the one exception —
 * it needs real DOM events, and works because the cards mount inside
 * aboveView's WCV which already holds keyboard focus during edit.
 */

import { memo } from 'react'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Menu } from '@base-ui/react/menu'
import type { CanvasSceneFileEntity, SelectionModifiers } from '../../shared/types'
import {
  RendererSwitch,
} from '../canvas-bg/entity-renderers/RendererSwitch'
import { getFileApi } from '../canvas-bg/entity-renderers/filePathToSrc'
import { CanvasViewportLayer, EntityShell } from './CanvasViewportLayer'

function FileBodyCard({
  entity,
  isDark,
  isSelected,
  isInteractive,
  canEdit,
  onTextEditingChange,
  onOpenLink,
}: {
  entity: CanvasSceneFileEntity
  isDark: boolean
  isSelected: boolean
  /** This is the entered interactive file (HTML iframe) — its content owns
   *  the pointer so scroll/clicks pass through. */
  isInteractive: boolean
  canEdit: boolean
  onTextEditingChange: (active: boolean) => void
  /** Open a link inside a markdown note as a page on the canvas. */
  onOpenLink: (id: string, url: string) => void
}) {
  const fileApi = getFileApi()

  // Bare entities (images, device-framed pages) show no card: transparent
  // background, no shadow, square corners.
  const isChromeless = entity.rendererTag === 'image' || entity.showDeviceFrame

  const menuPopupClass = `z-50 min-w-40 rounded-[10px] border p-1 shadow-xl outline-none ${
    isDark
      ? 'border-zinc-700 bg-zinc-900 text-[var(--surface-foreground)]'
      : 'border-zinc-200 bg-white text-[var(--surface-foreground)]'
  }`
  const menuItemClass = `flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
    isDark
      ? 'text-[var(--surface-foreground)] data-[highlighted]:bg-zinc-800'
      : 'text-[var(--surface-foreground)] data-[highlighted]:bg-zinc-100'
  }`

  return (
    <EntityShell
      id={entity.id}
      canvasX={entity.canvasX}
      canvasY={entity.canvasY}
      style={{
        width: entity.width,
        height: entity.height,
        background: isChromeless ? 'transparent' : isDark ? '#1c1917' : '#fafaf9',
        boxShadow: isChromeless
          ? undefined
          : isDark
            ? '0 2px 8px rgba(0, 0, 0, 0.3)'
            : '0 2px 8px rgba(0, 0, 0, 0.08)',
        overflow: isSelected ? 'visible' : 'hidden',
        borderRadius: isChromeless ? 0 : 4,
      }}
    >
      <ContextMenu.Root>
        <ContextMenu.Trigger className="block" style={{ width: '100%', height: '100%' }}>
          <RendererSwitch
            entity={entity}
            canEdit={canEdit}
            isDark={isDark}
            isInteractive={isInteractive}
            onTextEditingChange={onTextEditingChange}
            onOpenLink={onOpenLink}
          />
        </ContextMenu.Trigger>
        <Menu.Portal>
          <ContextMenu.Backdrop
            data-overlay-ui
            className="fixed inset-0 z-40"
          />
          <Menu.Positioner sideOffset={6} style={{ zIndex: 50 }}>
            <Menu.Popup data-overlay-ui className={menuPopupClass}>
              <Menu.Item
                className={menuItemClass}
                onClick={() => fileApi.showFileInFinder(entity.file)}
              >
                Show in Finder
              </Menu.Item>
              {entity.rendererTag === 'image' && (
                <Menu.Item
                  className={menuItemClass}
                  onClick={() => fileApi.copyFileAsPng(entity.file)}
                >
                  Copy as PNG
                </Menu.Item>
              )}
              {entity.rendererTag === 'html' && (
                <Menu.Item
                  className={menuItemClass}
                  onClick={() => fileApi.refreshFileEntity(entity.id)}
                >
                  Refresh
                </Menu.Item>
              )}
              <div
                role="separator"
                className={isDark ? 'my-1 h-px bg-zinc-700' : 'my-1 h-px bg-zinc-200'}
              />
              <Menu.Item
                className={menuItemClass}
                onClick={() => fileApi.reorderStack('bring-forward', entity.id)}
              >
                Bring forward
              </Menu.Item>
              <Menu.Item
                className={menuItemClass}
                onClick={() => fileApi.reorderStack('send-backward', entity.id)}
              >
                Send backward
              </Menu.Item>
              <Menu.Item
                className={menuItemClass}
                onClick={() => fileApi.reorderStack('bring-to-front', entity.id)}
              >
                Bring to front
              </Menu.Item>
              <Menu.Item
                className={menuItemClass}
                onClick={() => fileApi.reorderStack('send-to-back', entity.id)}
              >
                Send to back
              </Menu.Item>
            </Menu.Popup>
          </Menu.Positioner>
        </Menu.Portal>
      </ContextMenu.Root>
    </EntityShell>
  )
}

const MemoFileBodyCard = memo(FileBodyCard, (prev, next) => {
  return (
    prev.entity.id === next.entity.id &&
    prev.entity.file === next.entity.file &&
    prev.entity.fileReloadVersion === next.entity.fileReloadVersion &&
    prev.entity.subpath === next.entity.subpath &&
    prev.entity.canvasX === next.entity.canvasX &&
    prev.entity.canvasY === next.entity.canvasY &&
    prev.entity.width === next.entity.width &&
    prev.entity.height === next.entity.height &&
    prev.entity.objectFit === next.entity.objectFit &&
    prev.entity.showDeviceFrame === next.entity.showDeviceFrame &&
    prev.entity.deviceId === next.entity.deviceId &&
    prev.entity.deviceOrientation === next.entity.deviceOrientation &&
    prev.entity.rendererTag === next.entity.rendererTag &&
    prev.entity.noteContent === next.entity.noteContent &&
    prev.entity.componentHasRepo === next.entity.componentHasRepo &&
    prev.entity.componentInferredRepoPath === next.entity.componentInferredRepoPath &&
    prev.isDark === next.isDark &&
    prev.isSelected === next.isSelected &&
    prev.isInteractive === next.isInteractive &&
    prev.canEdit === next.canEdit
  )
})

export function FileBodyLayer({
  entities,
  isDark,
  selectedEntityIdSet,
  editingEntityId,
  interactiveEntityId,
  canvasOrigin,
  pan,
  zoom,
  onTextEditingChange,
  onOpenLink,
}: {
  entities: CanvasSceneFileEntity[]
  isDark: boolean
  selectedEntityIdSet: Set<string>
  /** id of the entity currently in inline-edit mode (or null). Mounts the
   *  inner editable surface iff `editingEntityId === entity.id`. */
  editingEntityId: string | null
  /** Entered interactive file (HTML iframe) whose content owns the pointer. */
  interactiveEntityId: string | null
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  onTextEditingChange: (active: boolean) => void
  /** Open a link inside a markdown note as a page on the canvas. */
  onOpenLink: (id: string, url: string) => void
}) {
  if (!entities.length) return null
  return (
    <CanvasViewportLayer canvasOrigin={canvasOrigin} pan={pan} zoom={zoom}>
      {entities.map((entity) => (
        <MemoFileBodyCard
          key={entity.id}
          entity={entity}
          isDark={isDark}
          isSelected={selectedEntityIdSet.has(entity.id)}
          isInteractive={interactiveEntityId === entity.id}
          canEdit={editingEntityId === entity.id}
          onTextEditingChange={onTextEditingChange}
          onOpenLink={onOpenLink}
        />
      ))}
    </CanvasViewportLayer>
  )
}
