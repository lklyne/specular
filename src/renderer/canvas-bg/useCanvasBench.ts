import { useEffect } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import { runCanvasBench } from './canvasBench'

type BenchApi = Pick<
  CanvasBgElectronAPI,
  'canvasPan' | 'canvasZoom' | 'onCanvasBenchRun' | 'sendCanvasBenchResult'
>

/**
 * Listens for main's benchmark request and drives it. A failed or refused run
 * still answers, with no runs, because main blocks on this reply — a silent
 * throw here would hang the benchmark rather than fail it.
 */
export function useCanvasBench(api: BenchApi): void {
  useEffect(() => {
    let running = false
    return api.onCanvasBenchRun((request) => {
      if (running) return
      running = true
      void runCanvasBench(
        {
          pan: (deltaX, deltaY) => api.canvasPan(deltaX, deltaY),
          zoom: (deltaY, mouseX, mouseY) => api.canvasZoom(deltaY, mouseX, mouseY),
        },
        request,
      )
        .then((result) => api.sendCanvasBenchResult(result))
        .catch(() => api.sendCanvasBenchResult({ refreshMs: 0, runs: [] }))
        .finally(() => {
          running = false
        })
    })
  }, [api])
}
