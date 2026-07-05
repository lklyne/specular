import { ipcChannels } from '../../shared/ipc-contract'
import { ipcMain } from 'electron'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import type { ScrollSyncData, SelectionModifiers } from '../../shared/types'
import { isAdditiveSelection } from '../../shared/selection-modifiers'
import { bgView } from '../runtime/view-refs'
import { zoom } from '../runtime/runtime-context'
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
    if (!page || !page.linked) return
    if (isScrollSuppressed(page)) return
    propagateScrollFromPage(page, data)
  })

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
