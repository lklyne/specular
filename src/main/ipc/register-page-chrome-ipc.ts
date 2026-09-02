import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import type {
  ElementAttachmentPositionsUpdate,
  InteractionSyncEvent,
  LocatorResolveResponse,
  ScrollSyncData,
  SelectionModifiers,
} from '../../shared/types'
import { isAdditiveSelection } from '../../shared/selection-modifiers'
import { bgView } from '../runtime/view-refs'
import { zoom } from '../runtime/runtime-context'
import { requestLayout } from '../runtime/viewport-control'
import { markDirty } from '../runtime/layout-dirty'
import {
  deselectAll,
} from '../runtime/ui-actions'
import {
  findPageByPageView,
} from '../runtime/page-runtime'
import { win } from '../runtime/window-shell'
import {
  isScrollSuppressed,
  propagateScrollFromPage,
} from '../navigation-sync'
import {
  handleInteractionSyncEvent,
  handleResolveInteractionLocatorResponse,
} from '../interaction-sync'
import { selectionDebug } from '../runtime/runtime-constants'
import { applyElementAttachmentPositions } from '../runtime/element-attachment-positions'
import { broadcastRuntimePatch } from '../runtime/runtime-patch-broadcast'
import { livePageScrollOffsets, pageScrollMovesScene } from '../runtime/page-scroll-state'

export function registerPageChromeIpc(): void {
  ipcMain.on(
    ipcChannels.pageDeselect,
    (_event, payload?: { modifiers?: SelectionModifiers }) => {
      // Additive modifiers (shift/meta/ctrl) preserve the existing selection
      // so clicking on empty space with a modifier held does not wipe it.
      if (isAdditiveSelection(payload?.modifiers)) {
        selectionDebug('ipc:page-deselect:suppressed-additive')
        return
      }
      selectionDebug('ipc:page-deselect')
      deselectAll()
    },
  )

  ipcMain.on(ipcChannels.pageScrollChanged, (event, data: ScrollSyncData) => {
    const page = findPageByPageView(event.sender)
    if (!page || !page.syncId) return
    if (isScrollSuppressed(page)) return
    propagateScrollFromPage(page, data)
  })

  // Always-on absolute-pixel scroll offset (ADR 0031 scroll amendment).
  // Unlike `pageScrollChanged` this has no `syncId` gate — every page reports
  // its offset so page-anchored regions can scroll-follow. Stored on the
  // ephemeral runtime page; the layout broadcast carries it.
  ipcMain.on(
    ipcChannels.pageScrollOffset,
    (event, data: { scrollX: number; scrollY: number; scrollHeight: number }) => {
      const page = findPageByPageView(event.sender)
      if (!page) return
      if (
        page.scrollX === data.scrollX &&
        page.scrollY === data.scrollY &&
        page.scrollHeight === data.scrollHeight
      ) {
        return
      }
      page.scrollX = data.scrollX
      page.scrollY = data.scrollY
      page.scrollHeight = data.scrollHeight
      // Scroll-following overlays (shapes, region annotations) get the raw
      // offset immediately and shift themselves with a CSS transform, instead
      // of waiting out the debounced layout rebuild — that multi-hop path lags
      // the page's native compositor scroll and reads as jitter.
      broadcastRuntimePatch({
        kind: 'slice',
        slice: 'pageScroll',
        value: livePageScrollOffsets(),
      })
      // A pass still has real work when the scene is bound to this document:
      // anchored entities shift by the scroll delta, and the page entity's
      // offset is what annotations are projected against. Nothing bound means
      // nothing left for it to compute.
      if (!pageScrollMovesScene(page.id)) return
      markDirty('canvas')
      requestLayout()
    },
  )

  // ADR 0032 — element-attachment reflow positions. The page's tracker
  // broadcasts the live document positions of its subscribed selectors on real
  // reflow events (resize, load, debounced mutations). Stored on the ephemeral
  // runtime page keyed by selector; scene builders read them as a render-time
  // correction. No patch of its own: every subscribed selector belongs to an
  // anchored item, so the pass has to run to fold the shift into that item's
  // geometry, and a patch would only duplicate what the pass already sends.
  ipcMain.on(
    ipcChannels.elementAttachmentPositions,
    (event, data: ElementAttachmentPositionsUpdate | undefined) => {
      if (!applyElementAttachmentPositions(event.sender, data)) return
      markDirty('canvas')
      requestLayout()
    },
  )

  ipcMain.on(ipcChannels.interactionSyncEvent, (event, payload: InteractionSyncEvent) => {
    handleInteractionSyncEvent(event.sender, payload)
  })

  ipcMain.on(
    ipcChannels.resolveInteractionLocatorResponse,
    (event, payload: LocatorResolveResponse) => {
      handleResolveInteractionLocatorResponse(event.sender, payload)
    },
  )

  ipcMain.on(ipcChannels.canvasBgDropdownOpen, () => {
    if (!bgView || !win) return
    requestLayout()
  })

  ipcMain.on(ipcChannels.canvasBgDropdownClose, () => {
    requestLayout()
  })

  ipcMain.on(ipcChannels.peekResizeStart, (event) => {
    const page = findPageByPageView(event.sender)
    if (!page) return
    const vp = VIEWPORT_PRESETS[page.presetIndex]
    page.peekWidth = vp.width
    page.peekHeight = vp.height
  })

  ipcMain.on(ipcChannels.peekResizeMove, (event, { dx, dy }: { dx: number; dy: number }) => {
    const page = findPageByPageView(event.sender)
    if (!page || page.peekWidth === undefined || page.peekHeight === undefined) return
    page.peekWidth = Math.max(320, Math.round(page.peekWidth + dx / zoom))
    page.peekHeight = Math.max(200, Math.round(page.peekHeight + dy / zoom))
    requestLayout()
  })

  ipcMain.on(ipcChannels.peekResizeEnd, (event) => {
    const page = findPageByPageView(event.sender)
    if (!page) return
    page.peekWidth = undefined
    page.peekHeight = undefined
    requestLayout()
  })
}
