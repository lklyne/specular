import path from 'path'
import type { Route } from './types'
import { writeJson } from './http-helpers'
import {
  getPerfTraceState,
  getTraceSummary,
  isPerfTraceRecording,
  listPerfTraces,
  startPerfTrace,
  stopPerfTrace,
} from '../perf-trace'

export const perfRoutes: Route[] = [
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
      if (isPerfTraceRecording()) {
        writeJson(response, 409, { error: 'Already recording' })
        return
      }
      await startPerfTrace()
      writeJson(response, 200, { recording: true })
    },
  },
  {
    method: 'POST',
    pattern: '/perf/trace/stop',
    async handler({ response, body }) {
      if (!isPerfTraceRecording()) {
        writeJson(response, 409, { error: 'Not recording' })
        return
      }
      const tracePath = await stopPerfTrace()
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
