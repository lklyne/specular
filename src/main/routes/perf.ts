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
import { sampleProcessMetrics } from '../process-metrics'
import { runVisibilityProbe } from '../visibility-probe'

export const perfRoutes: Route[] = [
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
      const result = await runPanZoomPerfTest()
      const payload = body as { summarize?: boolean }
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
  {
    method: 'GET',
    pattern: '/perf/metrics',
    async handler({ response }) {
      writeJson(response, 200, sampleProcessMetrics())
    },
  },
  {
    method: 'POST',
    pattern: '/perf/visibility-probe',
    async handler({ response, body }) {
      const payload = body as { windowMs?: number }
      writeJson(response, 200, await runVisibilityProbe({ windowMs: payload?.windowMs }))
    },
  },
]
