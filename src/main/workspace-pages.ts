import type {
  CreatePagesRequest,
  CreatePagesResponse,
  PageConfig,
  WorkspaceGroup,
} from '../shared/types'
import {
  CLUSTER_HORIZONTAL_GUTTER,
  CLUSTER_OUTER_MARGIN,
  USER_GROUP_PADDING,
  VIEWPORT_PRESETS,
} from '../shared/constants'
import { defaultOrientationForDevice, deviceForPresetIndex } from '../shared/device-catalog'
import {
  createPage,
  findPageById,
  pages,
} from './runtime/page-runtime'
import {
  getSelectedEntityIds,
  selectPageById,
  setSelectedEntities,
  setSelectedPages,
  setSelectedGroupId,
} from './runtime/ui-actions'
import { textEntities, createTextEntity as createTextEntityInState } from './runtime/text-entity-state'
import { fileEntities, createFileEntity as createFileEntityInState } from './runtime/file-entity-state'
import { shapeEntities, createShapeEntity as createShapeEntityInState } from './runtime/shape-entity-state'
import {
  drawingEntities,
  createDrawingEntity as createDrawingEntityInState,
} from './runtime/drawing-entity-state'
import {
  pageContentSize,
  requestLayout,
  snapToGrid,
} from './runtime/surface-layout'
import { scheduleWorkspaceAutosave } from './runtime/workspace-session'
import { setCustomPageSizeMetadata, setDeviceIdMetadata } from './runtime/runtime-entities'
import { makeId, cloneMetadata, pageCurrentUrl, createGroup } from './workspace-utils'
import {
  entityBoundsById,
  groupById,
  groupChildIds,
} from './workspace-entities'
import { findDuplicatePlacement } from './workspace-placement'
import { reflowManagedGroup } from './managed-layout'

// --- Helpers ---

function globalRightmostPlacement(
  presetIndex: number,
): Pick<PageConfig, 'canvasX' | 'canvasY'> {
  const maxRight = pages.reduce((rightmost, page) => {
    const size = pageContentSize(page)
    return Math.max(rightmost, page.canvasX + size.width + CLUSTER_HORIZONTAL_GUTTER)
  }, CLUSTER_OUTER_MARGIN)

  return {
    canvasX: snapToGrid(Math.max(CLUSTER_OUTER_MARGIN, maxRight)),
    canvasY: snapToGrid(CLUSTER_OUTER_MARGIN),
  }
}

function manualGroupLabel(url: string): string {
  try {
    const parsed = new URL(url)
    const path = parsed.pathname === '/' ? '' : parsed.pathname
    return `Pages: ${parsed.hostname}${path}`
  } catch {
    return 'Pages'
  }
}

function ensureManualRowGroup(sourcePageId: string, url: string): WorkspaceGroup {
  const existingPage = findPageById(sourcePageId)
  if (existingPage?.parentGroupId) {
    const existingGroup = groupById(existingPage.parentGroupId)
    if (existingGroup) return existingGroup
  }

  const sourceBounds = entityBoundsById(sourcePageId) ?? { x: 0, y: 0, width: 0, height: 0 }

  const group = createGroup({
    id: makeId('group'),
    kind: 'group',
    label: manualGroupLabel(url),
    canvasX: sourceBounds.x - USER_GROUP_PADDING,
    canvasY: sourceBounds.y - USER_GROUP_PADDING,
    width: sourceBounds.width + USER_GROUP_PADDING * 2,
    height: sourceBounds.height + USER_GROUP_PADDING * 2,
    parentGroupId: existingPage?.parentGroupId,
    layoutMode: 'row',
    managedLayout: true,
    metadata: {
      url,
      createdFrom: 'manual_row',
    },
  })

  if (existingPage) {
    existingPage.parentGroupId = group.id
  }

  return group
}

// --- Exported page operations ---

export function addPageFromSource(input: {
  sourcePageId?: string
  presetIndex: number
  customSize?: boolean
  focus?: boolean
}): { pageId: string; groupId?: string } {
  const preset = VIEWPORT_PRESETS[input.presetIndex]
  if (!preset) {
    throw new Error(`Unknown preset index: ${input.presetIndex}`)
  }

  const sourcePage = input.sourcePageId ? findPageById(input.sourcePageId) : undefined
  const url = pageCurrentUrl(sourcePage?.id) ?? 'about:blank'

  if (!sourcePage) {
    const placement = globalRightmostPlacement(input.presetIndex)
    const device = deviceForPresetIndex(input.presetIndex)
    const page = createPage({
      url,
      presetIndex: input.presetIndex,
      linked: false,
      canvasX: placement.canvasX,
      canvasY: placement.canvasY,
      source: 'manual',
      metadata: setDeviceIdMetadata(
        {
          createdFrom: 'add_from_toolbar',
          deviceOrientation: defaultOrientationForDevice(device),
          showDeviceFrame: true,
        },
        device?.id ?? null,
      ),
    })
    if (input.customSize) {
      page.metadata = setCustomPageSizeMetadata(page.metadata, pageContentSize(page))
    }
    if (input.focus ?? true) {
      selectPageById(page.id)
    }
    requestLayout()
    scheduleWorkspaceAutosave()
    return { pageId: page.id }
  }

  const group = sourcePage.parentGroupId
    ? groupById(sourcePage.parentGroupId) ?? ensureManualRowGroup(sourcePage.id, url)
    : ensureManualRowGroup(sourcePage.id, url)

  const fallbackDevice = deviceForPresetIndex(input.presetIndex)
  const metadata = setDeviceIdMetadata(
    { createdFrom: 'add_from_toolbar', deviceOrientation: defaultOrientationForDevice(fallbackDevice) },
    fallbackDevice?.id ?? null,
  )
  const newPage = createPage({
    url,
    presetIndex: input.presetIndex,
    linked: false,
    suppressInitialNavigationBroadcast: true,
    canvasX: sourcePage.canvasX,
    canvasY: sourcePage.canvasY,
    source: 'manual',
    parentGroupId: group.id,
    metadata,
  })
  if (input.customSize) {
    newPage.metadata = setCustomPageSizeMetadata(newPage.metadata, pageContentSize(newPage))
  }
  newPage.parentGroupId = group.id
  reflowManagedGroup(group.id)

  if (input.focus ?? true) {
    setSelectedGroupId(group.id)
    selectPageById(newPage.id)
  }
  requestLayout()
  scheduleWorkspaceAutosave()
  return { pageId: newPage.id, groupId: group.id }
}

export function createPageAtPosition(input: {
  sourcePageId?: string
  presetIndex: number
  customSize?: boolean
  canvasX: number
  canvasY: number
  mode: 'add_from_toolbar' | 'duplicate' | 'paste_url'
  focus?: boolean
  url?: string
}): { pageId: string } {
  const preset = VIEWPORT_PRESETS[input.presetIndex]
  if (!preset) {
    throw new Error(`Unknown preset index: ${input.presetIndex}`)
  }

  const url = input.url ?? pageCurrentUrl(input.sourcePageId) ?? 'about:blank'
  // Auto-assign device based on the preset so orientation tabs appear immediately
  const matchedDevice = deviceForPresetIndex(input.presetIndex)
  const metadata = setDeviceIdMetadata(
    {
      createdFrom: input.mode,
      deviceOrientation: defaultOrientationForDevice(matchedDevice),
      showDeviceFrame: true,
    },
    matchedDevice?.id ?? null,
  )

  const page = createPage({
    url,
    presetIndex: input.presetIndex,
    linked: false,
    canvasX: snapToGrid(input.canvasX),
    canvasY: snapToGrid(input.canvasY),
    source: 'manual',
    metadata,
  })
  if (input.customSize) {
    page.metadata = setCustomPageSizeMetadata(page.metadata, pageContentSize(page))
  }

  if (input.focus ?? true) {
    selectPageById(page.id)
  }
  requestLayout()
  scheduleWorkspaceAutosave()
  return { pageId: page.id }
}

export function duplicatePageFromSource(input: {
  sourcePageId: string
  focus?: boolean
  url?: string
}): { pageId: string } {
  const sourcePage = findPageById(input.sourcePageId)
  if (!sourcePage) {
    throw new Error(`Unknown page: ${input.sourcePageId}`)
  }

  // A caller-supplied url means we're opening a link as a frame, not
  // duplicating the page in place: keep the source's preset/size/device, but
  // drop url-specific overrides (injected CSS/localStorage/props keyed to the
  // source page) so they don't bleed onto an unrelated destination.
  const isLinkOpen = input.url !== undefined
  const url = input.url ?? pageCurrentUrl(sourcePage.id) ?? 'about:blank'
  const clonedMetadata = cloneMetadata(sourcePage.metadata) ?? {}
  if (isLinkOpen) delete clonedMetadata.overrides
  const metadata = {
    ...clonedMetadata,
    createdFrom: isLinkOpen ? 'link' : 'duplicate',
  }
  const sourceSize = pageContentSize(sourcePage)
  const placement = findDuplicatePlacement({
    x: sourcePage.canvasX,
    y: sourcePage.canvasY,
    width: sourceSize.width,
    height: sourceSize.height,
  })
  const newPage = createPage({
    url,
    presetIndex: sourcePage.presetIndex,
    linked: false,
    suppressInitialNavigationBroadcast: true,
    canvasX: placement.canvasX,
    canvasY: placement.canvasY,
    source: 'manual',
    parentGroupId: sourcePage.parentGroupId,
    metadata,
  })
  if (input.focus ?? true) {
    selectPageById(newPage.id)
  }
  requestLayout()
  scheduleWorkspaceAutosave()
  return { pageId: newPage.id }
}

export function createPages(input: CreatePagesRequest): CreatePagesResponse {
  const pageIds: string[] = []
  for (const config of input.pages) {
    const page = createPage(config)
    pageIds.push(page.id)
  }
  requestLayout()
  if (pageIds.length) scheduleWorkspaceAutosave()
  return { pageIds }
}

export function tidySelectedPages(): { pageIds: string[] } {
  const selectedPageIds = getSelectedEntityIds()
  if (!selectedPageIds.length) return { pageIds: [] }

  const pagesToTidy = selectedPageIds
    .map((pageId) => findPageById(pageId))
    .filter(
      (
        page,
      ): page is Exclude<ReturnType<typeof findPageById>, undefined> =>
        page !== undefined,
    )

  if (!pagesToTidy.length) return { pageIds: [] }

  pagesToTidy.sort((a, b) => {
    const aSize = pageContentSize(a)
    const bSize = pageContentSize(b)
    const areaDelta = aSize.width * aSize.height - bSize.width * bSize.height
    if (areaDelta !== 0) return areaDelta
    const widthDelta = aSize.width - bSize.width
    if (widthDelta !== 0) return widthDelta
    return a.id.localeCompare(b.id)
  })

  const startX = snapToGrid(Math.min(...pagesToTidy.map((page) => page.canvasX)))
  const endX = snapToGrid(
    Math.max(
      ...pagesToTidy.map((page) => page.canvasX + pageContentSize(page).width),
    ),
  )
  const startY = snapToGrid(Math.min(...pagesToTidy.map((page) => page.canvasY)))
  const totalWidth = pagesToTidy.reduce(
    (sum, page) => sum + pageContentSize(page).width,
    0,
  )
  const gapCount = Math.max(0, pagesToTidy.length - 1)
  const availableGapWidth = Math.max(0, endX - startX - totalWidth)
  const distributedGap = gapCount > 0 ? availableGapWidth / gapCount : 0

  let cursorX = startX
  for (const page of pagesToTidy) {
    const { width } = pageContentSize(page)
    page.canvasX = cursorX
    page.canvasY = startY
    cursorX = page.canvasX + width + distributedGap
  }

  requestLayout()
  scheduleWorkspaceAutosave()
  return { pageIds: pagesToTidy.map((page) => page.id) }
}

export function duplicateEntity(input: {
  entityId: string
  focus?: boolean
}): { entityId: string } {
  const page = findPageById(input.entityId)
  if (page) {
    const result = duplicatePageFromSource({
      sourcePageId: page.id,
      focus: input.focus,
    })
    return { entityId: result.pageId }
  }

  const note = textEntities.find((n) => n.id === input.entityId)
  if (note) {
    const notePlacement = findDuplicatePlacement({
      x: note.canvasX,
      y: note.canvasY,
      width: note.width,
      height: note.height,
    })
    const newNote = createTextEntityInState({
      canvasX: notePlacement.canvasX,
      canvasY: notePlacement.canvasY,
      text: note.text,
      color: note.color,
      textStyle: note.textStyle,
      widthMode: note.widthMode,
      textSize: note.textSize,
      width: note.width,
      height: note.height,
    })
    if (input.focus ?? true) {
      setSelectedEntities([newNote.id])
    }
    requestLayout()
    scheduleWorkspaceAutosave()
    return { entityId: newNote.id }
  }

  const file = fileEntities.find((f) => f.id === input.entityId)
  if (file) {
    const filePlacement = findDuplicatePlacement({
      x: file.canvasX,
      y: file.canvasY,
      width: file.width,
      height: file.height,
    })
    const newFile = createFileEntityInState({
      canvasX: filePlacement.canvasX,
      canvasY: filePlacement.canvasY,
      file: file.file,
      subpath: file.subpath,
      width: file.width,
      height: file.height,
      presetIndex: file.presetIndex,
      metadata: file.metadata ? { ...file.metadata } : undefined,
      objectFit: file.objectFit,
    })
    if (input.focus ?? true) {
      setSelectedEntities([newFile.id])
    }
    requestLayout()
    scheduleWorkspaceAutosave()
    return { entityId: newFile.id }
  }

  const shape = shapeEntities.find((s) => s.id === input.entityId)
  if (shape) {
    const shapePlacement = findDuplicatePlacement({
      x: shape.canvasX,
      y: shape.canvasY,
      width: shape.width,
      height: shape.height,
    })
    const newShape = createShapeEntityInState({
      canvasX: shapePlacement.canvasX,
      canvasY: shapePlacement.canvasY,
      shapeKind: shape.shapeKind,
      text: shape.text,
      color: shape.color,
      strokeWidth: shape.strokeWidth,
      textSize: shape.textSize,
      theme: shape.theme,
      width: shape.width,
      height: shape.height,
      label: shape.label,
    })
    if (input.focus ?? true) {
      setSelectedEntities([newShape.id])
    }
    requestLayout()
    scheduleWorkspaceAutosave()
    return { entityId: newShape.id }
  }

  const drawing = drawingEntities.find((d) => d.id === input.entityId)
  if (drawing) {
    const drawingPlacement = findDuplicatePlacement({
      x: drawing.canvasX,
      y: drawing.canvasY,
      width: drawing.width,
      height: drawing.height,
    })
    const dx = drawingPlacement.canvasX - drawing.canvasX
    const dy = drawingPlacement.canvasY - drawing.canvasY
    const newDrawing = createDrawingEntityInState({
      canvasX: drawingPlacement.canvasX,
      canvasY: drawingPlacement.canvasY,
      width: drawing.width,
      height: drawing.height,
      strokes: drawing.strokes.map((stroke) => ({
        ...stroke,
        id: `${stroke.id}_dup_${Math.random().toString(36).slice(2, 8)}`,
        points: stroke.points.map((p) => ({ x: p.x + dx, y: p.y + dy })),
      })),
      label: drawing.label,
    })
    if (input.focus ?? true) {
      setSelectedEntities([newDrawing.id])
    }
    requestLayout()
    scheduleWorkspaceAutosave()
    return { entityId: newDrawing.id }
  }

  throw new Error(`Unknown entity: ${input.entityId}`)
}
