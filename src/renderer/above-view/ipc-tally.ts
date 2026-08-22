/**
 * TEMPORARY — main→above-view IPC instrumentation for the mouse-move memory
 * growth investigation. Wraps every `api.on*` subscription and, every 2s,
 * logs per-channel delivery count + payload bytes, the JS heap, and (for the
 * dominant `layoutUpdate` channel) a per-key size breakdown, via console.warn
 * so it lands in `~/Library/Logs/Specular/errors.log`.
 *
 * Wire it by wrapping the api once at module load:
 *   const api = withIpcTally(window.electronAPI)
 *
 * Delete with the investigation.
 */
export function withIpcTally<T extends object>(source: T): T {
  const api: Record<string, unknown> = { ...(source as Record<string, unknown>) }
  const counts = new Map<string, { n: number; bytes: number; maxBytes: number }>()
  let keySizesLogged = false

  for (const key of Object.keys(api)) {
    if (!key.startsWith('on') || typeof api[key] !== 'function') continue
    const orig = api[key] as (cb: (p: unknown) => void) => () => void
    api[key] = (cb: (p: unknown) => void) =>
      orig((payload) => {
        let bytes = 0
        try {
          bytes = JSON.stringify(payload)?.length ?? 0
        } catch {
          bytes = -1
        }
        if (key === 'onLayoutUpdate' && payload && typeof payload === 'object' && !keySizesLogged) {
          keySizesLogged = true
          const sizes = Object.entries(payload as Record<string, unknown>)
            .map(([k, v]) => {
              let n = 0
              try {
                n = JSON.stringify(v)?.length ?? 0
              } catch {
                /* circular */
              }
              return [k, n] as const
            })
            .sort((a, b) => b[1] - a[1])
            .slice(0, 12)
            .map(([k, n]) => `${k}=${(n / 1024).toFixed(0)}KB`)
          console.warn(`[ipc-tally] layoutUpdate keys: ${sizes.join(' ')}`)
        }
        const row = counts.get(key) ?? { n: 0, bytes: 0, maxBytes: 0 }
        row.n += 1
        row.bytes += bytes
        row.maxBytes = Math.max(row.maxBytes, bytes)
        counts.set(key, row)
        cb(payload)
      })
  }

  setInterval(() => {
    if (counts.size === 0) return
    const mem = (performance as unknown as { memory?: { usedJSHeapSize: number; totalJSHeapSize: number } }).memory
    const rows = [...counts.entries()].map(
      ([k, v]) => `${k}=${v.n}x/${(v.bytes / 1024).toFixed(0)}KB(max ${(v.maxBytes / 1024).toFixed(0)}KB)`,
    )
    console.warn(
      `[ipc-tally] heapUsed=${((mem?.usedJSHeapSize ?? 0) / 1048576).toFixed(0)}MB heapTotal=${((mem?.totalJSHeapSize ?? 0) / 1048576).toFixed(0)}MB ${rows.join(' ')}`,
    )
    counts.clear()
  }, 2000)

  return api as T
}
