import path from 'path'
import type { Route } from './types'
import { writeJson } from './http-helpers'
import {
  getPerfTraceState,
  getPerfTraceOwner,
  getTraceSummary,
  isPerfTraceRecording,
  listPerfTraces,
  startPerfTrace,
  stopPerfTrace,
} from '../perf-trace'
import {
  getPanZoomPerfTestState,
  runPanZoomPerfTest,
  stopPanZoomPerfTest,
} from '../pan-zoom-perf-test'
import type { PanZoomPerfPhase } from '../../shared/pan-zoom-perf-test'
import { captureWindowFramesWhile } from '../window-frame-capture'
import { win } from '../runtime/view-refs'
import { requestLayout } from '../runtime/layout-engine'
import {
  clearZoomSnapshotFreeze,
  isZoomSnapshotHiResEnabled,
  prepareZoomSnapshotFreeze,
  setZoomSnapshotHiResEnabled,
  setZoomSnapshotFreezeActive,
  showPreparedZoomSnapshots,
} from '../runtime/zoom-snapshot-freeze'
import { fitAllPagesForBench, runZoomSnapshotBench } from '../runtime/zoom-snapshot-bench'
import { captureViaCdp } from '../runtime/zoom-snapshot-cdp-capture'
import { pages, zoom } from '../runtime/runtime-context'
import { boundEffectivePageContentSize } from '../runtime/runtime-geometry'
import { app, screen } from 'electron'
import { writeFile } from 'fs/promises'
import type { ZoomSnapshotBenchVariant } from '../../shared/types'
import {
  isZoomSceneRebroadcastEnabled,
  setZoomSceneRebroadcast,
} from '../runtime/zoom-motion'

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

export const perfRoutes: Route[] = [
  {
    method: 'GET',
    pattern: '/perf/window-id',
    async handler({ response }) {
      writeJson(response, 200, { windowId: win?.getMediaSourceId().split(':')[1] ?? null })
    },
  },
  {
    // Diagnostic: one renderer-side hi-res capture per visible page, with the
    // output size and timing; `save` writes the JPEGs to the logs folder.
    method: 'POST',
    pattern: '/perf/zoom-snapshot/cdp-probe',
    async handler({ response, body }) {
      const payload = (body ?? {}) as { scale?: number; quality?: number; save?: boolean }
      const scale = payload.scale ?? 1
      const results: unknown[] = []
      for (const page of pages) {
        if (page.pageView.webContents.isDestroyed()) continue
        const bounds = page.pageView.getBounds()
        if (bounds.width <= 0 || bounds.height <= 0) continue
        const css = boundEffectivePageContentSize(page)
        try {
          const capture = await captureViaCdp(page, {
            scale,
            quality: payload.quality,
            cssWidth: css.width,
            cssHeight: css.height,
            emulation: { deviceScaleFactor: screen.getPrimaryDisplay().scaleFactor, scale: zoom },
          })
          let file: string | null = null
          if (payload.save) {
            file = path.join(app.getPath('logs'), `cdp-probe-${page.id}-s${scale}.jpg`)
            await writeFile(file, capture.jpeg)
          }
          results.push({
            pageId: page.id,
            zoom,
            css,
            viewBounds: { width: bounds.width, height: bounds.height },
            outWidth: capture.width,
            outHeight: capture.height,
            bytes: capture.jpeg.byteLength,
            ms: Math.round(capture.ms),
            file,
          })
        } catch (error) {
          results.push({ pageId: page.id, error: String(error) })
        }
      }
      writeJson(response, 200, { scale, results })
    },
  },
  {
    method: 'POST',
    pattern: '/perf/flags',
    async handler({ response, body }) {
      const payload = (body ?? {}) as { zoomSceneRebroadcast?: boolean; zoomSnapshotHiRes?: boolean }
      if (typeof payload.zoomSceneRebroadcast === 'boolean') {
        setZoomSceneRebroadcast(payload.zoomSceneRebroadcast)
      }
      if (typeof payload.zoomSnapshotHiRes === 'boolean') {
        setZoomSnapshotHiResEnabled(payload.zoomSnapshotHiRes)
      }
      writeJson(response, 200, {
        zoomSceneRebroadcast: isZoomSceneRebroadcastEnabled(),
        zoomSnapshotHiRes: isZoomSnapshotHiResEnabled(),
      })
    },
  },
  {
    method: 'POST',
    pattern: '/perf/zoom-snapshot/bench',
    async handler({ response, body }) {
      const payload = body as {
        variants?: ZoomSnapshotBenchVariant[]
        repeats?: number
        fit?: boolean
        zoom?: number
        /** Drop the prepared frames first, to exercise the cold-start path. */
        clear?: boolean
      }
      if (payload.clear) clearZoomSnapshotFreeze()
      const repeats = Math.max(1, Math.min(10, payload.repeats ?? 1))
      if (payload.fit) {
        fitAllPagesForBench({ zoom: payload.zoom })
        // Let the zoom settle, views re-emulate, and pages re-raster.
        await wait(1_500)
      }
      const runs = []
      for (let i = 0; i < repeats; i += 1) {
        runs.push(await runZoomSnapshotBench({ variants: payload.variants }))
      }
      writeJson(response, 200, { runs })
    },
  },
  {
    method: 'GET',
    pattern: '/perf/pan-zoom/status',
    async handler({ response }) {
      writeJson(response, 200, getPanZoomPerfTestState())
    },
  },
  {
    method: 'POST',
    pattern: '/perf/pan-zoom/run',
    async handler({ response, body }) {
      if (isPerfTraceRecording() && !getPanZoomPerfTestState().running) {
        writeJson(response, 409, { error: 'A performance trace is already active' })
        return
      }
      const payload = body as { summarize?: boolean; profiles?: string[]; durationMs?: number }
      const result = await runPanZoomPerfTest({
        phaseIds: payload.profiles as PanZoomPerfPhase['id'][] | undefined,
        durationMs: payload.durationMs,
      })
      const summary = payload.summarize
        ? await getTraceSummary(result.fileName)
        : undefined
      writeJson(response, 200, { ...result, summary })
    },
  },
  {
    method: 'POST',
    pattern: '/perf/pan-zoom/stop',
    async handler({ response }) {
      await stopPanZoomPerfTest()
      writeJson(response, 200, getPanZoomPerfTestState())
    },
  },
  {
    method: 'POST',
    pattern: '/perf/pan-zoom/visual-run',
    async handler({ response, body }) {
      if (isPerfTraceRecording() && !getPanZoomPerfTestState().running) {
        writeJson(response, 409, { error: 'A performance trace is already active' })
        return
      }
      const payload = body as {
        summarize?: boolean
        profiles?: string[]
        durationMs?: number
        snapshotFreeze?: boolean
      }
      const snapshotPreparation = payload.snapshotFreeze
        ? await prepareZoomSnapshotFreeze()
        : undefined
      const capture = await captureWindowFramesWhile(async () => {
        if (snapshotPreparation) {
          // Give canvas-bg time to decode and paint while live views still cover
          // it, then park the native views before starting the measured trace.
          showPreparedZoomSnapshots()
          await wait(80)
          setZoomSnapshotFreezeActive(true)
          requestLayout()
          await wait(50)
        }
        try {
          return await runPanZoomPerfTest({
            phaseIds: payload.profiles as PanZoomPerfPhase['id'][] | undefined,
            durationMs: payload.durationMs,
          })
        } finally {
          if (snapshotPreparation) {
            // Restore live views underneath the frozen layer first so there is
            // no blank transition, then release the encoded images.
            setZoomSnapshotFreezeActive(false)
            requestLayout()
            await wait(50)
            clearZoomSnapshotFreeze()
          }
        }
      })
      const summary = payload.summarize
        ? await getTraceSummary(capture.result.fileName)
        : undefined
      writeJson(response, 200, {
        ...capture.result,
        frameDirectory: capture.frameDirectory,
        frameCount: capture.samples.length,
        manifestPath: capture.manifestPath,
        snapshotPreparation,
        summary,
      })
    },
  },
  {
    method: 'GET',
    pattern: '/perf/trace/status',
    async handler({ response }) {
      writeJson(response, 200, getPerfTraceState())
    },
  },
  {
    method: 'POST',
    pattern: '/perf/trace/start',
    async handler({ response }) {
      const state = getPerfTraceState()
      if (state.status !== 'idle') {
        writeJson(response, 409, { error: 'Already recording' })
        return
      }
      await startPerfTrace({ revealOnAutoStop: false })
      writeJson(response, 200, { recording: true })
    },
  },
  {
    method: 'POST',
    pattern: '/perf/trace/stop',
    async handler({ response, body }) {
      const state = getPerfTraceState()
      if (state.status === 'idle') {
        writeJson(response, 409, { error: 'Not recording' })
        return
      }
      if (state.status === 'starting') {
        writeJson(response, 409, { error: 'Trace is still starting' })
        return
      }
      if (getPerfTraceOwner() !== 'manual') {
        writeJson(response, 409, { error: 'Trace belongs to the pan/zoom test; stop the test instead' })
        return
      }
      const tracePath = await stopPerfTrace({ reveal: false, owner: 'manual' })
      if (!tracePath) {
        writeJson(response, 500, { error: 'Failed to stop trace' })
        return
      }
      const fileName = path.basename(tracePath)
      const payload = body as { summarize?: boolean }
      const summary = payload.summarize ? await getTraceSummary(fileName) : undefined
      writeJson(response, 200, { tracePath, fileName, summary })
    },
  },
  {
    method: 'GET',
    pattern: '/perf/traces',
    async handler({ response }) {
      writeJson(response, 200, await listPerfTraces())
    },
  },
  {
    method: 'GET',
    pattern: /^\/perf\/trace\/summary(\?.*)?$/,
    async handler({ response, url }) {
      const searchParams = new URL(url, 'http://localhost').searchParams
      const file = searchParams.get('file')
      if (!file) {
        writeJson(response, 400, { error: 'file query param is required' })
        return
      }
      const summary = await getTraceSummary(file)
      if (!summary) {
        writeJson(response, 404, { error: `Trace not found or unreadable: ${file}` })
        return
      }
      writeJson(response, 200, summary)
    },
  },
]
