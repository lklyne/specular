/**
 * `page` entity kind — a live web item (ADR 0003).
 *
 * Owns device-metadata prep on create (the `preparedPageCreates` pass that used
 * to live in `entity-ops.ts`) and the preset/orientation/device-frame/url
 * handling that used to live in the `/pages/update` route.
 */

import {
  LAPTOP_PRESET_INDEX,
  VIEWPORT_PRESETS,
  defaultOrientationForDevice,
  deviceForPresetIndex,
} from '../../../shared/device-catalog'
import type { PersistedPageEntity } from '../../../shared/types'
import type { JsonCanvasLinkNode } from '../../../shared/json-canvas-types'
import { normalizeUserUrl } from '../../../shared/url'
import { navigatePage } from '../../navigation-sync'
import { createPages } from '../../workspace-pages'
import { deletePages } from '../../workspace-entities'
import { findPageById, pages } from '../../runtime/runtime-context'
import {
  setDeviceOrientation,
  setPagePreset,
} from '../../runtime/document-commands'
import {
  setShowDeviceFrameMetadata,
  showDeviceFrameFromMetadata,
} from '../../runtime/runtime-entities'
import {
  deserializeLinkNodeToPage,
  serializePageToLinkNode,
} from '../../runtime/json-canvas-serializer'
import type { EntityKindDefinition } from '../contract'

export const pageKind: EntityKindDefinition<'page'> = {
  kind: 'page',
  fields: ['canvasX', 'canvasY', 'presetIndex', 'orientation', 'showDeviceFrame', 'url'],

  create(input) {
    const presetIndex = (input.presetIndex as number | undefined) ?? LAPTOP_PRESET_INDEX
    const device = deviceForPresetIndex(presetIndex)
    const orientation = (input.orientation as string | undefined)
      ?? defaultOrientationForDevice(device)
    const metadata: Record<string, unknown> = {
      ...((input.metadata as Record<string, unknown> | undefined) ?? {}),
      deviceId: device?.id ?? null,
      deviceOrientation: orientation,
      showDeviceFrame: input.showDeviceFrame !== false,
    }
    const { pageIds } = createPages({
      pages: [{
        id: input.id as string | undefined,
        name: input.name as string | undefined,
        url: normalizeUserUrl((input.url as string | undefined) ?? ''),
        presetIndex,
        canvasX: (input.canvasX as number | undefined) ?? 0,
        canvasY: (input.canvasY as number | undefined) ?? 0,
        linked: input.linked as boolean | undefined,
        groupId: input.groupId as string | undefined,
        parentGroupId: input.parentGroupId as string | undefined,
        metadata,
      }],
    })
    return pageIds[0]
  },

  update(id, patch) {
    const page = findPageById(id)
    if (!page) return
    if (patch.presetIndex !== undefined) setPagePreset(page.id, patch.presetIndex as number)
    if (patch.orientation !== undefined) {
      setDeviceOrientation(page.id, patch.orientation as 'portrait' | 'landscape')
    }
    if (patch.showDeviceFrame !== undefined) {
      const next = patch.showDeviceFrame as boolean
      if (showDeviceFrameFromMetadata(page.metadata) !== next) {
        page.metadata = setShowDeviceFrameMetadata(page.metadata, next)
      }
    }
    if (patch.url !== undefined && patch.url !== page.url) {
      navigatePage(page, { type: 'load-url', url: patch.url as string })
    }
    if (patch.canvasX !== undefined) page.canvasX = patch.canvasX as number
    if (patch.canvasY !== undefined) page.canvasY = patch.canvasY as number
  },

  delete(id) {
    return deletePages({ pageIds: [id] }).deletedPageIds.length > 0
  },

  serialize(entity) {
    return serializePageToLinkNode(entity as PersistedPageEntity)
  },

  deserialize(node) {
    return deserializeLinkNodeToPage(node as JsonCanvasLinkNode)
  },

  defaultSize(input) {
    const presetIndex = (input.presetIndex as number | undefined) ?? LAPTOP_PRESET_INDEX
    const preset = VIEWPORT_PRESETS[presetIndex] ?? VIEWPORT_PRESETS[LAPTOP_PRESET_INDEX]
    return { width: preset?.width ?? 1280, height: preset?.height ?? 800 }
  },

  entities: () => pages,
}
