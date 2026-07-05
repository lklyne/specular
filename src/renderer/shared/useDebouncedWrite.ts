import { useCallback, useEffect, useMemo, useRef } from 'react'

/**
 * Debounced write for inline editors: `schedule(value)` queues `write(value)`
 * after `delayMs` of quiet, replacing any still-pending value.
 *
 * `cancel()` drops the pending write (callers that commit synchronously on
 * blur use it so the stale debounced value can't land after the commit).
 * `isPending()` lets refresh paths (visibility change, external-change
 * events) skip re-fetching while a local write is still queued.
 *
 * With `flushOnUnmount`, a pending write runs in the unmount cleanup so the
 * last keystrokes before a tab switch or entity deletion aren't lost.
 * Without it, a pending timer is left to fire on its own.
 */
export function useDebouncedWrite(
  write: (value: string) => void,
  { delayMs = 300, flushOnUnmount = false }: { delayMs?: number; flushOnUnmount?: boolean } = {},
) {
  const writeRef = useRef(write)
  writeRef.current = write
  const delayRef = useRef(delayMs)
  delayRef.current = delayMs
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const flushRef = useRef<(() => void) | null>(null)

  const schedule = useCallback((value: string) => {
    const doWrite = writeRef.current
    flushRef.current = () => doWrite(value)
    if (timeoutRef.current) clearTimeout(timeoutRef.current)
    timeoutRef.current = setTimeout(() => {
      flushRef.current?.()
      flushRef.current = null
      timeoutRef.current = null
    }, delayRef.current)
  }, [])

  const cancel = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
      timeoutRef.current = null
    }
    flushRef.current = null
  }, [])

  const isPending = useCallback(() => timeoutRef.current !== null, [])

  const flushOnUnmountRef = useRef(flushOnUnmount)
  flushOnUnmountRef.current = flushOnUnmount
  useEffect(() => {
    return () => {
      if (!flushOnUnmountRef.current) return
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
        timeoutRef.current = null
        flushRef.current?.()
        flushRef.current = null
      }
    }
  }, [])

  // Stable identity so the hook result can sit in dependency arrays.
  return useMemo(() => ({ schedule, cancel, isPending }), [schedule, cancel, isPending])
}
