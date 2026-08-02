import type {
  CreatePagesRequest,
  CreatePagesResponse,
  PageConfig,
  WorkspaceGroup,
} from '../shared/types'
import {
  CLUSTER_HORIZONTAL_GUTTER,
  CLUSTER_OUTER_MARGIN,
  DESKTOP_PRESET_INDEX,
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
  setSelectedPages,
  setSelectedGroupId,
} from './runtime/ui-actions'
import { pageContentSize } from './runtime/runtime-geometry'
import { snapToGrid } from '../shared/gesture-utils'
import {
  focusCanvasBounds,
  recenterFocusPresentation,
} from './runtime/viewport-control'
import { mutateWorkspace } from './runtime/mutate-workspace'
import { setCustomPageSizeMetadata, setDeviceIdMetadata } from './runtime/runtime-entities'
import { focusSession, repointFocusSession } from './runtime/focus-session'
import { makeId, cloneMetadata, pageCurrentUrl, createGroup } from './workspace-utils'
import {
  entityBoundsById,
  groupById,
  groupChildIds,
} from './workspace-entities'
import {
  findDuplicatePlacement,
  findPlacement,
  findPlacementBeside,
} from './workspace-placement'
import { createEdges } from './workspace-edges'
import { normalizeUserUrl } from '../shared/url'
import { reflowManagedGroup } from './managed-layout'
import { copyableEntityPayload, pasteEntitiesFromClipboard } from './workspace-clipboard'

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
  return mutateWorkspace(() => addPageFromSourceInternal(input))
}

function addPageFromSourceInternal(input: {
  sourcePageId?: string
  presetIndex: number
  customSize?: boolean
  focus?: boolean
}): { pageId: string; groupId?: string } {

  const sourcePage = input.sourcePageId ? findPageById(input.sourcePageId) : undefined
  const url = pageCurrentUrl(sourcePage?.id) ?? 'about:blank'

  if (!sourcePage) {
    const placement = globalRightmostPlacement(input.presetIndex)
    const device = deviceForPresetIndex(input.presetIndex)
    const page = createPage({
      url,
      presetIndex: input.presetIndex,
      syncId: null,
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
    syncId: null,
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

  return mutateWorkspace(() => {
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
      syncId: null,
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
    return { pageId: page.id }
  })
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
  return mutateWorkspace(() => duplicatePageInternal(input, sourcePage))
}

function duplicatePageInternal(
  input: { sourcePageId: string; focus?: boolean; url?: string },
  sourcePage: NonNullable<ReturnType<typeof findPageById>>,
): { pageId: string } {
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
    syncId: null,
    suppressInitialNavigationBroadcast: true,
    canvasX: placement.canvasX,
    canvasY: placement.canvasY,
    source: 'manual',
    parentGroupId: sourcePage.parentGroupId,
    metadata,
    colorScheme: sourcePage.colorScheme,
  })
  const focusNewPage = input.focus ?? true
  if (focusNewPage) {
    selectPageById(newPage.id)
  }
  // Opening a link as a frame via a regular click glides the camera to center
  // the new frame (keeping zoom). Background opens (cmd/middle-click) and plain
  // duplicates leave the camera where it is.
  if (isLinkOpen && focusNewPage) {
    focusCanvasBounds(
      {
        x: placement.canvasX,
        y: placement.canvasY,
        width: sourceSize.width,
        height: sourceSize.height,
      },
      { animate: true },
    )
  }
  return { pageId: newPage.id }
}

export function createBlankFrameFromSource(input: {
  sourcePageId: string
}): { pageId: string } {
  const sourcePage = findPageById(input.sourcePageId)
  if (!sourcePage) {
    throw new Error(`Unknown page: ${input.sourcePageId}`)
  }
  return mutateWorkspace(() => createBlankFrameInternal(sourcePage))
}

function createBlankFrameInternal(
  sourcePage: NonNullable<ReturnType<typeof findPageById>>,
): { pageId: string } {
  const sourceSize = pageContentSize(sourcePage)
  const placement = findDuplicatePlacement({
    x: sourcePage.canvasX,
    y: sourcePage.canvasY,
    width: sourceSize.width,
    height: sourceSize.height,
  })
  const clonedMetadata = cloneMetadata(sourcePage.metadata) ?? {}
  delete clonedMetadata.overrides
  const metadata = {
    ...clonedMetadata,
    createdFrom: 'new-frame',
  }

  const newPage = createPage({
    url: 'about:blank',
    presetIndex: sourcePage.presetIndex,
    syncId: null,
    suppressInitialNavigationBroadcast: true,
    canvasX: placement.canvasX,
    canvasY: placement.canvasY,
    source: 'manual',
    parentGroupId: sourcePage.parentGroupId,
    metadata,
    colorScheme: sourcePage.colorScheme,
  })

  selectPageById(newPage.id)
  if (focusSession()?.pageId === sourcePage.id) {
    repointFocusSession(newPage.id)
    recenterFocusPresentation(newPage.id)
  } else {
    focusCanvasBounds(
      {
        x: placement.canvasX,
        y: placement.canvasY,
        width: sourceSize.width,
        height: sourceSize.height,
      },
      { animate: true },
    )
  }

  return { pageId: newPage.id }
}

export function createPages(input: CreatePagesRequest): CreatePagesResponse {
  return mutateWorkspace(() => {
    const pageIds: string[] = []
    for (const config of input.pages) {
      const page = createPage(config)
      pageIds.push(page.id)
    }
    return { pageIds }
  }, { changed: (result) => result.pageIds.length > 0 })
}

export function tidySelectedPages(): { pageIds: string[] } {
  return mutateWorkspace(
    () => tidySelectedPagesInternal(),
    { changed: (result) => result.pageIds.length > 0 },
  )
}

function tidySelectedPagesInternal(): { pageIds: string[] } {
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

  return { pageIds: pagesToTidy.map((page) => page.id) }
}

/**
 * Single-entity duplicate (cmd-D, per-kind context-menu "Duplicate"). Clones
 * through the same copy/paste machinery `pasteEntitiesFromClipboard` uses
 * for clipboard paste and drag-copy, rather than a parallel per-kind clone —
 * one clone mechanism instead of three (see workspace-clipboard.ts,
 * workspace-groups.ts for the other two entity-graph clone paths).
 */
/**
 * Trailing-slash-insensitive form for "is a page already showing this URL".
 * Deliberately not the hash-insensitive `canonicalPageUrl` from
 * `src/shared/page-anchor.ts`: a trailing slash is the one difference this
 * match tolerates.
 */
function linkMatchUrl(value: string | null | undefined): string | null {
  if (!value) return null
  try {
    return normalizeUserUrl(value).replace(/\/$/, '')
  } catch {
    return null
  }
}

/**
 * Open a link found inside an entity's text as a page on the canvas. A page
 * already showing the URL is revealed instead of duplicated; otherwise a new
 * page opens beside the source entity, connected to it with an edge so the
 * note visibly owns its reference.
 */
export function openLinkFromEntity(input: {
  entityId: string
  url: string
}): { pageId: string } | null {
  let url: string
  try {
    url = normalizeUserUrl(input.url)
  } catch {
    return null
  }
  const canonical = linkMatchUrl(url)

  const existing = pages.find(
    (page) => linkMatchUrl(pageCurrentUrl(page.id) ?? page.url) === canonical,
  )
  if (existing) {
    selectPageById(existing.id)
    const size = pageContentSize(existing)
    focusCanvasBounds(
      { x: existing.canvasX, y: existing.canvasY, width: size.width, height: size.height },
      { animate: true },
    )
    return { pageId: existing.id }
  }

  const preset = VIEWPORT_PRESETS[DESKTOP_PRESET_INDEX]
  const anchor = entityBoundsById(input.entityId)
  const placement = anchor
    ? findPlacementBeside(anchor, preset.width, preset.height)
    : findPlacement({
        width: preset.width,
        height: preset.height,
        anchor: 'selection_or_empty_region',
      })
  const { pageId } = createPageAtPosition({
    presetIndex: DESKTOP_PRESET_INDEX,
    canvasX: placement.canvasX,
    canvasY: placement.canvasY,
    mode: 'paste_url',
    focus: true,
    url,
  })
  if (anchor) {
    createEdges({
      edges: [
        {
          fromEntityId: input.entityId,
          toEntityId: pageId,
          fromSide: 'right',
          toSide: 'left',
          kind: 'connection',
        },
      ],
    })
  }
  focusCanvasBounds(
    { x: placement.canvasX, y: placement.canvasY, width: preset.width, height: preset.height },
    { animate: true },
  )
  return { pageId }
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
  return duplicateEntityInternal(input)
}

// `pasteEntitiesFromClipboard` is itself a `mutateWorkspace` mutator, so this
// stays a plain function — wrapping it again would double the undo boundary
// and autosave scheduling for a single cmd-D.
function duplicateEntityInternal(input: {
  entityId: string
  focus?: boolean
}): { entityId: string } {
  const payload = copyableEntityPayload([input.entityId])
  const bounds = entityBoundsById(input.entityId)
  if (!payload || !bounds) {
    throw new Error(`Unknown entity: ${input.entityId}`)
  }
  const placement = findDuplicatePlacement(bounds)
  const result = pasteEntitiesFromClipboard({
    payload,
    canvasX: placement.canvasX,
    canvasY: placement.canvasY,
  })
  const entityId = result.entityIds[0]
  if (!entityId) {
    throw new Error(`Unknown entity: ${input.entityId}`)
  }
  return { entityId }
}
