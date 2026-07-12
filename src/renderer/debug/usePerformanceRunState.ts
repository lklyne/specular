import { useEffect, useState } from 'react'
import type {
  DebugElectronAPI,
  PerfTraceState,
} from '../../shared/electron-api/debug'
import type { PanZoomPerfTestState } from '../../shared/pan-zoom-perf-test'

const RECORD_MAX_SECONDS = 30

const IDLE_TRACE_STATE: PerfTraceState = {
  recording: false,
  status: 'idle',
  startedAt: null,
}

const IDLE_PAN_ZOOM_STATE: PanZoomPerfTestState = {
  running: false,
  stopping: false,
  phase: null,
  startedAt: null,
}

export function usePerformanceRunState(
  api: DebugElectronAPI,
  onTraceSaved: () => void,
): {
  traceState: PerfTraceState
  panZoomState: PanZoomPerfTestState
} {
  const [traceState, setTraceState] = useState(IDLE_TRACE_STATE)
  const [panZoomState, setPanZoomState] = useState(IDLE_PAN_ZOOM_STATE)

  useEffect(() => {
    let cancelled = false
    void Promise.all([api.perfTraceGetState(), api.perfPanZoomGetState()]).then(
      ([nextTraceState, nextPanZoomState]) => {
        if (cancelled) return
        setTraceState(nextTraceState)
        setPanZoomState(nextPanZoomState)
      },
    )
    return () => {
      cancelled = true
    }
  }, [api])

  useEffect(
    () =>
      api.onPerfTraceStateChanged((next) => {
        setTraceState((previous) => {
          if (previous.status === 'stopping' && next.status === 'idle') onTraceSaved()
          return next
        })
      }),
    [api, onTraceSaved],
  )

  useEffect(
    () => api.onPerfPanZoomStateChanged(setPanZoomState),
    [api],
  )

  return { traceState, panZoomState }
}

export function useTraceElapsedSeconds(traceState: PerfTraceState): number {
  const [elapsedSeconds, setElapsedSeconds] = useState(0)

  useEffect(() => {
    if (!traceState.recording || traceState.startedAt === null) {
      setElapsedSeconds(0)
      return
    }
    const startedAt = traceState.startedAt
    const tick = () =>
      setElapsedSeconds(
        Math.min(RECORD_MAX_SECONDS, Math.floor((Date.now() - startedAt) / 1000)),
      )
    tick()
    const id = setInterval(tick, 250)
    return () => clearInterval(id)
  }, [traceState.recording, traceState.startedAt])

  return elapsedSeconds
}
