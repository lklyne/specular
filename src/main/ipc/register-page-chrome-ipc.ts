import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import type { ScrollSyncData, SelectionModifiers } from '../../shared/types'
import { isAdditiveSelection } from '../../shared/selection-modifiers'
import { aboveView, bgView } from '../runtime/view-refs'
import { safeSend } from '../runtime/safe-send'
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
import { selectionDebug } from '../runtime/runtime-constants'

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

  // Always-on absolute-pixel scroll offset (docs/plans/scroll-tracking.md).
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
      // Fast path: scroll-following overlays (shapes, region annotations) get
      // the raw offset immediately and shift themselves with a CSS transform,
      // instead of waiting out the debounced layout rebuild below — that
      // multi-hop path lags the page's native compositor scroll and reads as
      // jitter. The full broadcast still follows and reconciles.
      if (aboveView) {
        safeSend(aboveView.webContents, ipcChannels.pageScrollLive, {
          pageId: page.id,
          scrollX: data.scrollX,
          scrollY: data.scrollY,
        })
      }
      markDirty('canvas')
      requestLayout()
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
