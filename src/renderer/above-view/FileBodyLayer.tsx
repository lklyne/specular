/**
 * FileBodyLayer — file-entity bodies (image, video, markdown, wireframe,
 * component placeholder, fallback). Mounted in aboveView so a file placed
 * over a page is actually drawn above it.
 *
 * Hit-tests run in `useCanvasPointerRouter` against the layout snapshot
 * (front-to-back), so this layer is purely visual for selection/drag/resize.
 * The contenteditable inside markdown / wireframe renderers is the one
 * exception — it needs real DOM events, and works because the cards mount
 * inside aboveView's WCV which already holds keyboard focus during edit.
 */

import { memo } from 'react'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Menu } from '@base-ui/react/menu'
import type { CanvasSceneFileEntity, SelectionModifiers } from '../../shared/types'
import {
  RendererSwitch,
} from '../canvas-bg/entity-renderers/RendererSwitch'
import { getFileApi } from '../canvas-bg/entity-renderers/filePathToSrc'

/**
 * Wraps the file cards in a viewport transform so they live in
 * canvas-coordinate space. AboveView's WCV origin already sits at
 * `canvasOrigin.y` (the toolbar inset), so the translate omits that axis
 * — only `canvasOrigin.x` and `pan` apply. Matches `StickyViewportLayer`
 * and `ShapeViewportLayer`.
 */
function FileViewportLayer({
  canvasOrigin,
  pan,
  zoom,
  children,
}: {
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  children: React.ReactNode
}) {
  return (
    <div
      className="pointer-events-none absolute left-0 top-0 origin-top-left"
      style={{
        ['--canvas-zoom' as string]: zoom,
        transform: `translate(${canvasOrigin.x + pan.x}px, ${pan.y}px) scale(${zoom})`,
      }}
    >
      {children}
    </div>
  )
}

function FileShell({
  id,
  canvasX,
  canvasY,
  width,
  height,
  isDark,
  isSelected,
  background,
  borderRadius,
  showCardShadow,
  children,
}: {
  id: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  isDark: boolean
  isSelected: boolean
  background: string
  borderRadius?: number
  showCardShadow: boolean
  children: React.ReactNode
}) {
  return (
    <div
      data-entity-id={id}
      className="absolute pointer-events-auto"
      style={{
        left: canvasX,
        top: canvasY,
        width,
        height,
        background,
        boxShadow: showCardShadow
          ? isDark
            ? '0 2px 8px rgba(0, 0, 0, 0.3)'
            : '0 2px 8px rgba(0, 0, 0, 0.08)'
          : undefined,
        overflow: isSelected ? 'visible' : 'hidden',
        cursor: 'default',
        borderRadius,
        touchAction: 'none',
      }}
    >
      {children}
    </div>
  )
}

function FileBodyCard({
  entity,
  isDark,
  isSelected,
  canEdit,
  wireframeJsonMode,
  onTextEditingChange,
}: {
  entity: CanvasSceneFileEntity
  isDark: boolean
  isSelected: boolean
  canEdit: boolean
  wireframeJsonMode: boolean
  onTextEditingChange: (active: boolean) => void
}) {
  const fileApi = getFileApi()

  const menuPopupClass = `z-50 min-w-40 rounded-[10px] border p-1 shadow-xl outline-none ${
    isDark
      ? 'border-zinc-700 bg-zinc-900 text-zinc-100'
      : 'border-zinc-200 bg-white text-zinc-900'
  }`
  const menuItemClass = `flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
    isDark
      ? 'text-zinc-100 data-[highlighted]:bg-zinc-800'
      : 'text-zinc-900 data-[highlighted]:bg-zinc-100'
  }`

  return (
    <FileShell
      id={entity.id}
      canvasX={entity.canvasX}
      canvasY={entity.canvasY}
      width={entity.width}
      height={entity.height}
      isDark={isDark}
      isSelected={isSelected}
      background={entity.showDeviceFrame ? 'transparent' : isDark ? '#1c1917' : '#fafaf9'}
      borderRadius={entity.showDeviceFrame ? 0 : 4}
      showCardShadow={!entity.showDeviceFrame}
    >
      <ContextMenu.Root>
        <ContextMenu.Trigger className="block" style={{ width: '100%', height: '100%' }}>
          <RendererSwitch
            entity={entity}
            canEdit={canEdit}
            isDark={isDark}
            isSelected={isSelected}
            wireframeJsonMode={wireframeJsonMode}
            onTextEditingChange={onTextEditingChange}
          />
        </ContextMenu.Trigger>
        <Menu.Portal>
          <Menu.Positioner sideOffset={6} style={{ zIndex: 50 }}>
            <Menu.Popup data-overlay-ui className={menuPopupClass}>
              <Menu.Item
                className={menuItemClass}
                onClick={() => fileApi.showFileInFinder(entity.file)}
              >
                Show in Finder
              </Menu.Item>
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
    </FileShell>
  )
}

const MemoFileBodyCard = memo(FileBodyCard, (prev, next) => {
  return (
    prev.entity.id === next.entity.id &&
    prev.entity.file === next.entity.file &&
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
    prev.entity.componentHasRepo === next.entity.componentHasRepo &&
    prev.entity.componentInferredRepoPath === next.entity.componentInferredRepoPath &&
    prev.isDark === next.isDark &&
    prev.isSelected === next.isSelected &&
    prev.canEdit === next.canEdit &&
    prev.wireframeJsonMode === next.wireframeJsonMode
  )
})

/** Map of entityId → wireframe jsonMode. AboveView owns this state now. */
export type FileJsonModeMap = Map<string, boolean>

export function FileBodyLayer({
  entities,
  isDark,
  selectedEntityIdSet,
  editingEntityId,
  jsonModeMap,
  canvasOrigin,
  pan,
  zoom,
  onTextEditingChange,
}: {
  entities: CanvasSceneFileEntity[]
  isDark: boolean
  selectedEntityIdSet: Set<string>
  /** id of the entity currently in inline-edit mode (or null). Mounts the
   *  inner editable surface iff `editingEntityId === entity.id`. */
  editingEntityId: string | null
  jsonModeMap: FileJsonModeMap
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  onTextEditingChange: (active: boolean) => void
}) {
  if (!entities.length) return null
  return (
    <FileViewportLayer canvasOrigin={canvasOrigin} pan={pan} zoom={zoom}>
      {entities.map((entity) => (
        <MemoFileBodyCard
          key={entity.id}
          entity={entity}
          isDark={isDark}
          isSelected={selectedEntityIdSet.has(entity.id)}
          canEdit={editingEntityId === entity.id}
          wireframeJsonMode={jsonModeMap.get(entity.id) ?? false}
          onTextEditingChange={onTextEditingChange}
        />
      ))}
    </FileViewportLayer>
  )
}
