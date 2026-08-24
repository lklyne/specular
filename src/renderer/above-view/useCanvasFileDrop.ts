import type { LayoutSnapshotRef } from '../shared/hooks/useProjectedLayoutRef'
import { useEffect } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { screenPointToCanvasPoint, snapToGrid } from '../../shared/gesture-utils'

export function useCanvasFileDrop({
  api,
  layoutRef,
}: {
  api: Pick<CanvasBgElectronAPI, 'dropComponentFile' | 'dropFileBuffer'>
  layoutRef: LayoutSnapshotRef
}) {
  useEffect(() => {
    const handleDragOver = (event: DragEvent) => {
      if (!event.dataTransfer?.types.includes('Files')) return
      event.preventDefault()
      event.dataTransfer.dropEffect = 'copy'
    }

    const handleDrop = (event: DragEvent) => {
      const layout = layoutRef.current
      if (!event.dataTransfer?.files.length) return
      event.preventDefault()
      event.stopImmediatePropagation()

      const point = screenPointToCanvasPoint(event.clientX, event.clientY, layout)
      const canvasX = snapToGrid(point.x)
      const canvasY = snapToGrid(point.y)

      Array.from(event.dataTransfer.files).forEach((file, i) => {
        const ext = file.name.split('.').pop()?.toLowerCase() ?? 'png'
        if (ext === 'tsx' || ext === 'jsx') {
          api.dropComponentFile(file, canvasX + i * 20, canvasY + i * 20)
          return
        }
        const reader = new FileReader()
        reader.onload = () => {
          if (!reader.result) return
          const buffer = new Uint8Array(reader.result as ArrayBuffer)
          api.dropFileBuffer(buffer, ext, canvasX + i * 20, canvasY + i * 20)
        }
        reader.readAsArrayBuffer(file)
      })
    }

    document.addEventListener('dragover', handleDragOver)
    document.addEventListener('drop', handleDrop)
    return () => {
      document.removeEventListener('dragover', handleDragOver)
      document.removeEventListener('drop', handleDrop)
    }
  }, [api, layoutRef])
}
