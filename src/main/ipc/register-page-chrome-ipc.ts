import { ipcMain } from 'electron'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import type { ScrollSyncData, SelectionModifiers } from '../../shared/types'
import { isAdditiveSelection } from '../../shared/selection-modifiers'
import {
  bgView,
  zoom,
} from '../runtime/surface-layout'
import { requestLayout } from '../runtime/viewport-control'
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
    'page-deselect',
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

  // No 'page-hover' handler: aboveView's hit-test is the sole hover authority
  // (its gate is open by default per gate-predicate.ts), so the page's forwarded
  // hover events are intentionally dropped — an unhandled ipcMain.on does that.

  ipcMain.on('page-scroll-changed', (event, data: ScrollSyncData) => {
    const page = findPageByPageView(event.sender)
    if (!page || !page.linked) return
    if (isScrollSuppressed(page)) return
    propagateScrollFromPage(page, data)
  })

  ipcMain.on('canvas-bg-dropdown-open', () => {
    if (!bgView || !win) return
    requestLayout()
  })

  ipcMain.on('canvas-bg-dropdown-close', () => {
    requestLayout()
  })

  ipcMain.on('peek-resize-start', (event) => {
    const page = findPageByPageView(event.sender)
    if (!page) return
    const vp = VIEWPORT_PRESETS[page.presetIndex]
    page.peekWidth = vp.width
    page.peekHeight = vp.height
  })

  ipcMain.on('peek-resize-move', (event, { dx, dy }: { dx: number; dy: number }) => {
    const page = findPageByPageView(event.sender)
    if (!page || page.peekWidth === undefined || page.peekHeight === undefined) return
    page.peekWidth = Math.max(320, Math.round(page.peekWidth + dx / zoom))
    page.peekHeight = Math.max(200, Math.round(page.peekHeight + dy / zoom))
    requestLayout()
  })

  ipcMain.on('peek-resize-end', (event) => {
    const page = findPageByPageView(event.sender)
    if (!page) return
    page.peekWidth = undefined
    page.peekHeight = undefined
    requestLayout()
  })
}
