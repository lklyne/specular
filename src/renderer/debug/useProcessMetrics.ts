/**
 * Polls the main process for `app.getAppMetrics()` snapshots.
 *
 * `percentCPUUsage` and `idleWakeupsPerSecond` are both deltas since the
 * previous `getAppMetrics()` call, so the first sample after a pause reads 0
 * and the interval doubles as the averaging window. Longer intervals give
 * calmer, more trustworthy numbers.
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import type { DebugElectronAPI } from '../../shared/electron-api/debug'
import type { ProcessMetricsSample } from '../../shared/process-metrics'

export function useProcessMetrics(
  api: DebugElectronAPI,
  /** Poll period in ms, or null to hold the last sample. */
  intervalMs: number | null,
) {
  const [sample, setSample] = useState<ProcessMetricsSample | null>(null)
  const [error, setError] = useState<string | null>(null)
  const inFlight = useRef(false)

  const refresh = useCallback(async () => {
    if (inFlight.current) return
    inFlight.current = true
    try {
      setSample(await api.processMetricsSample())
      setError(null)
    } catch {
      setError('Could not read process metrics.')
    } finally {
      inFlight.current = false
    }
  }, [api])

  useEffect(() => {
    void refresh()
    if (intervalMs === null) return
    const timer = setInterval(() => void refresh(), intervalMs)
    return () => clearInterval(timer)
  }, [refresh, intervalMs])

  return { sample, error, refresh }
}
