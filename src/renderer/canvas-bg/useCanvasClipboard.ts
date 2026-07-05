import { useEffect } from 'react'
import type { RefObject } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import {
  isTypingTarget,
  screenPointToCanvasPoint,
  snapToGrid,
} from '../../shared/gesture-utils'

export function useCanvasClipboard(input: {
  api: CanvasBgElectronAPI
  layoutRef: RefObject<LayoutUpdateData>
}) {
  const { api, layoutRef } = input

  useEffect(() => {
    const shouldHijack = (event: ClipboardEvent): boolean => {
      if (isTypingTarget(event.target)) return false
      const sel = window.getSelection()
      if (sel && sel.toString().length > 0) return false
      return true
    }

    const handleCopy = (event: ClipboardEvent) => {
      if (!shouldHijack(event)) return
      if (!layoutRef.current.selectedEntityIds.length) return
      event.preventDefault()
      api.copySelection()
    }

    const handleCut = (event: ClipboardEvent) => {
      if (!shouldHijack(event)) return
      const layout = layoutRef.current
      if (!layout.selection.length) return
      event.preventDefault()
      if (layout.selectedEntityIds.length) api.copySelection()
      api.deleteSelectedEntities()
    }

    const handlePaste = (event: ClipboardEvent) => {
      if (!shouldHijack(event)) return
      const layout = layoutRef.current
      const rightInset = layout.devtoolsOpen ? layout.devtoolsWidth : 0
      const leftInset = layout.leftChromeWidth
      const centerX = leftInset + (window.innerWidth - rightInset - leftInset) / 2
      const centerY =
        layout.canvasOrigin.y + (window.innerHeight - layout.canvasOrigin.y) / 2
      const point = screenPointToCanvasPoint(centerX, centerY, layout)
      event.preventDefault()
      api.pasteSelection(snapToGrid(point.x), snapToGrid(point.y))
    }

    document.addEventListener('copy', handleCopy)
    document.addEventListener('cut', handleCut)
    document.addEventListener('paste', handlePaste)
    return () => {
      document.removeEventListener('copy', handleCopy)
      document.removeEventListener('cut', handleCut)
      document.removeEventListener('paste', handlePaste)
    }
  }, [api, layoutRef])
}
