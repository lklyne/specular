/**
 * File Entity State
 *
 * Manages the in-memory state of file entities on the canvas.
 * File entities reference external files (images, attachments) —
 * no Electron views, no browser runtime. They have position, size,
 * a file path, and an optional subpath.
 */

import { randomUUID } from 'crypto'
import { existsSync } from 'fs'
import { dirname } from 'path'
import type {
  CanvasSceneFileEntity,
  PersistedFileEntity,
} from '../../shared/types'
import { CUSTOM_SHELL_INSETS, shellInsetsForDevice } from '../../shared/device-catalog'
import { markDirty } from './layout-dirty'
import {
  deviceIdFromMetadata,
  deviceOrientationFromMetadata,
  showDeviceFrameFromMetadata,
} from './runtime-entities'
import { pickRenderer } from '../plugins/registry'
import { findRepoForPath } from './dev-server-manager'
import {
  watchEntityFile,
  unwatchEntityFile,
  teardownAllFileWatchers,
  getFileReloadVersion,
} from './local-file-watcher'

export type FileObjectFit = 'contain' | 'cover' | 'fill'

export interface FileEntity {
  id: string
  file: string
  subpath?: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  parentGroupId?: string
  objectFit?: FileObjectFit
  presetIndex?: number
  metadata?: Record<string, unknown>
}

export const DEFAULT_FILE_WIDTH = 300
export const DEFAULT_FILE_HEIGHT = 300

export const fileEntities: FileEntity[] = []

export function createFileEntity(input: {
  canvasX: number
  canvasY: number
  file: string
  subpath?: string
  width?: number
  height?: number
  id?: string
  parentGroupId?: string
  presetIndex?: number
  metadata?: Record<string, unknown>
  objectFit?: FileObjectFit
}): FileEntity {
  const entity: FileEntity = {
    id: input.id ?? `file_${randomUUID()}`,
    file: input.file,
    subpath: input.subpath,
    canvasX: input.canvasX,
    canvasY: input.canvasY,
    width: input.width ?? DEFAULT_FILE_WIDTH,
    height: input.height ?? DEFAULT_FILE_HEIGHT,
    parentGroupId: input.parentGroupId,
    presetIndex: input.presetIndex,
    metadata: input.metadata,
    objectFit: input.objectFit,
  }
  fileEntities.push(entity)
  watchEntityFile(entity.id, entity.file)
  markDirty('canvas', 'sidebar')
  return entity
}

export function updateFileEntity(id: string, patch: Partial<Omit<FileEntity, 'id'>>): FileEntity | null {
  const entity = fileEntities.find((e) => e.id === id)
  if (!entity) return null
  if (patch.file !== undefined && patch.file !== entity.file) {
    unwatchEntityFile(entity.id, entity.file)
    entity.file = patch.file
    watchEntityFile(entity.id, entity.file)
  }
  if (patch.subpath !== undefined) entity.subpath = patch.subpath
  if (patch.canvasX !== undefined) entity.canvasX = patch.canvasX
  if (patch.canvasY !== undefined) entity.canvasY = patch.canvasY
  if (patch.width !== undefined) entity.width = patch.width
  if (patch.height !== undefined) entity.height = patch.height
  if (patch.parentGroupId !== undefined) entity.parentGroupId = patch.parentGroupId
  if (patch.objectFit !== undefined) entity.objectFit = patch.objectFit
  if (patch.presetIndex !== undefined) entity.presetIndex = patch.presetIndex
  if (patch.metadata !== undefined) entity.metadata = patch.metadata
  markDirty('canvas', 'sidebar')
  return entity
}

export function deleteFileEntity(id: string): boolean {
  const idx = fileEntities.findIndex((e) => e.id === id)
  if (idx === -1) return false
  const entity = fileEntities[idx]
  fileEntities.splice(idx, 1)
  unwatchEntityFile(entity.id, entity.file)
  markDirty('canvas', 'sidebar')
  return true
}

export function clearFileEntities(): void {
  fileEntities.length = 0
  teardownAllFileWatchers()
}

export function buildFileEntitySceneEntity(
  entity: FileEntity,
  zoom: number,
  pan: { x: number; y: number },
  canvasOrigin: { x: number; y: number },
): CanvasSceneFileEntity {
  const contentScreenX = canvasOrigin.x + entity.canvasX * zoom + pan.x
  const contentScreenY = canvasOrigin.y + entity.canvasY * zoom + pan.y
  const contentScreenW = entity.width * zoom
  const contentScreenH = entity.height * zoom

  const deviceId = deviceIdFromMetadata(entity.metadata)
  const orientation = deviceOrientationFromMetadata(entity.metadata)
  const showShell = showDeviceFrameFromMetadata(entity.metadata)

  // Compute shell-inflated outer bounds
  let shellInsets: { top: number; right: number; bottom: number; left: number } | null = null
  if (showShell) {
    if (deviceId) {
      shellInsets = shellInsetsForDevice(deviceId, orientation)
    } else {
      shellInsets = CUSTOM_SHELL_INSETS
    }
  }

  const screenX = shellInsets ? contentScreenX - shellInsets.left * zoom : contentScreenX
  const screenY = shellInsets ? contentScreenY - shellInsets.top * zoom : contentScreenY
  const screenWidth = shellInsets ? contentScreenW + (shellInsets.left + shellInsets.right) * zoom : contentScreenW
  const screenHeight = shellInsets ? contentScreenH + (shellInsets.top + shellInsets.bottom) * zoom : contentScreenH

  return {
    kind: 'file',
    id: entity.id,
    file: entity.file,
    subpath: entity.subpath,
    canvasX: entity.canvasX,
    canvasY: entity.canvasY,
    width: entity.width,
    height: entity.height,
    parentGroupId: entity.parentGroupId,
    objectFit: entity.objectFit,
    deviceId,
    deviceOrientation: orientation,
    showDeviceFrame: showShell,
    screenX,
    screenY,
    screenWidth,
    screenHeight,
    contentScreenX: showShell ? contentScreenX : undefined,
    contentScreenY: showShell ? contentScreenY : undefined,
    contentScreenWidth: showShell ? contentScreenW : undefined,
    contentScreenHeight: showShell ? contentScreenH : undefined,
    fileReloadVersion: getFileReloadVersion(entity.id),
    ...rendererSceneFields(entity),
  }
}

function rendererSceneFields(entity: FileEntity): {
  rendererTag: CanvasSceneFileEntity['rendererTag']
  rendererEditable: CanvasSceneFileEntity['rendererEditable']
  popupContributions: CanvasSceneFileEntity['popupContributions']
  componentHasRepo: CanvasSceneFileEntity['componentHasRepo']
  componentInferredRepoPath: CanvasSceneFileEntity['componentInferredRepoPath']
} {
  const claim = pickRenderer(persistFileEntity(entity))
  const tag = claim?.rendererTag ?? undefined
  const rendererEditable = claim?.editable ?? false
  const contributions = claim?.popupContributionTags
  const popupContributions =
    contributions && contributions.length > 0
      ? (contributions as CanvasSceneFileEntity['popupContributions'])
      : undefined
  if (tag !== 'component') {
    return {
      rendererTag: tag,
      rendererEditable,
      popupContributions,
      componentHasRepo: undefined,
      componentInferredRepoPath: undefined,
    }
  }
  const hasRepo = findRepoForPath(entity.file) !== null
  return {
    rendererTag: tag,
    rendererEditable,
    popupContributions,
    componentHasRepo: hasRepo,
    componentInferredRepoPath: hasRepo ? undefined : inferRepoRoot(entity.file),
  }
}

/**
 * Walk up from a component file looking for the nearest package.json. Used
 * to suggest a one-click reconnect target on the placeholder when the file
 * isn't claimed by any currently-connected repo. Capped at 8 levels so a
 * pathological deep path can't sit in a long fs.existsSync loop on every
 * scene rebuild.
 */
function inferRepoRoot(filePath: string): string | undefined {
  let dir = dirname(filePath)
  for (let i = 0; i < 8; i++) {
    if (existsSync(`${dir}/package.json`)) return dir
    const parent = dirname(dir)
    if (parent === dir) return undefined
    dir = parent
  }
  return undefined
}

export function persistFileEntity(entity: FileEntity): PersistedFileEntity {
  return {
    kind: 'file',
    id: entity.id,
    file: entity.file,
    subpath: entity.subpath,
    canvasX: entity.canvasX,
    canvasY: entity.canvasY,
    width: entity.width,
    height: entity.height,
    parentGroupId: entity.parentGroupId,
    objectFit: entity.objectFit,
    presetIndex: entity.presetIndex,
    metadata: entity.metadata,
  }
}
