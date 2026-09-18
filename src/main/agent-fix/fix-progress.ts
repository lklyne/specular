/**
 * In-memory, ephemeral progress log per annotation fix.
 *
 * Streaming events from the Claude Agent SDK fix run land here so the
 * Comments panel can show what the agent is doing in real time. Entries persist
 * past completion so the user can still open the log after the fix finishes;
 * they are discarded when a new fix starts on the same annotation.
 */

import type {
  FixProgressEntry,
  FixProgressEvent,
  FixProgressEventKind,
  FixProgressStatus,
} from '../../shared/types'

const MAX_EVENTS_PER_ENTRY = 200
const NOTIFY_THROTTLE_MS = 120

const entries = new Map<string, FixProgressEntry>()

type ChangeListener = () => void
const listeners = new Set<ChangeListener>()

let notifyScheduled = false
let cachedSnapshot: Record<string, FixProgressEntry> | null = null

export function onProgressChange(fn: ChangeListener): () => void {
  listeners.add(fn)
  return () => listeners.delete(fn)
}

function scheduleNotify(): void {
  cachedSnapshot = null
  if (notifyScheduled) return
  notifyScheduled = true
  setTimeout(() => {
    notifyScheduled = false
    for (const fn of listeners) {
      try { fn() } catch (err) { console.error('fix-progress listener error:', err) }
    }
  }, NOTIFY_THROTTLE_MS)
}

export function startFixProgress(annotationId: string, origin: string): void {
  const now = new Date().toISOString()
  entries.set(annotationId, {
    annotationId,
    origin,
    startedAt: now,
    updatedAt: now,
    status: 'running',
    events: [],
  })
  scheduleNotify()
}

export function appendFixEvent(
  annotationId: string,
  kind: FixProgressEventKind,
  text: string,
): void {
  const entry = entries.get(annotationId)
  if (!entry) return
  const event: FixProgressEvent = {
    kind,
    text,
    timestamp: new Date().toISOString(),
  }
  entry.events.push(event)
  if (entry.events.length > MAX_EVENTS_PER_ENTRY) {
    entry.events.splice(0, entry.events.length - MAX_EVENTS_PER_ENTRY)
  }
  entry.updatedAt = event.timestamp
  scheduleNotify()
}

export function finalizeFixProgress(
  annotationId: string,
  status: FixProgressStatus,
  payload?: { summary?: string; shouldResolve?: boolean; error?: string },
): void {
  const entry = entries.get(annotationId)
  if (!entry) return
  entry.status = status
  entry.updatedAt = new Date().toISOString()
  if (payload?.summary != null) entry.summary = payload.summary
  if (payload?.shouldResolve != null) entry.shouldResolve = payload.shouldResolve
  if (payload?.error != null) entry.error = payload.error
  scheduleNotify()
}

export function getFixProgress(): Record<string, FixProgressEntry> {
  if (cachedSnapshot) return cachedSnapshot
  const snapshot: Record<string, FixProgressEntry> = {}
  for (const [id, entry] of entries) {
    snapshot[id] = {
      ...entry,
      events: entry.events.slice(),
    }
  }
  cachedSnapshot = snapshot
  return snapshot
}
