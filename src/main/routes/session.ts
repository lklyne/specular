import type { IncomingMessage } from 'http'
import type { Route } from './types'
import type {
  PresenceActivity,
  PresenceLabelKey,
  PresenceSurface,
  PresenceTargetQuery,
  PresenceTargetRect,
  PresenceTargetRefSource,
} from '../../shared/types'
import {
  getPresenceCursors,
  coercePresenceLabelKey,
  coercePresenceActivity,
  coercePresenceSurface,
  coercePresenceTargetRefSource,
  upsertPresenceCursor,
  upsertActivePresenceTask,
  clearActivePresenceTask,
  scheduleThinkingState,
  beginPresenceDeparture,
} from '../presence-cursor'
import {
  pendingIntents,
  PENDING_INTENT_TTL_MS,
  resolveCanvasPointForPage,
  resolvePresenceTargetRect,
  findPresenceTarget,
  setAgentBrowserRefs,
  invalidateAgentBrowserRefsForPage,
  synthesizeAgentBrowserTargetQuery,
  type PendingIntent,
  type QueuedPresenceIntent,
} from '../presence-manager'
import { mcpSessions, resolveSession } from '../presence-session'
import { invalidateAgentSnapshot, onAgentSnapshotInvalidated } from '../runtime/agent-snapshot-cache'
import { cdpProxyRegistrations } from '../cdp-proxy'
import { writeJson, notifyStatusListeners } from './http-helpers'

// The agent-browser ref cache goes stale on exactly the same events that
// invalidate main's own agent-snapshot cache (navigation, DOM churn) — wire
// it here rather than teaching agent-snapshot-cache.ts about presence.
onAgentSnapshotInvalidated(invalidateAgentBrowserRefsForPage)

function coercePresenceTargetQuery(value: unknown): PresenceTargetQuery | null {
  if (!value || typeof value !== 'object') return null
  const payload = value as Record<string, unknown>
  const selector = typeof payload.selector === 'string' ? payload.selector : null
  const text = typeof payload.text === 'string' ? payload.text : null
  const name = typeof payload.name === 'string' ? payload.name : null
  if (!selector && !text) return null
  return { selector, text, name }
}

/** Free-text task label from `specular presence start` / the MCP
 *  `start_task` tool — rendered on the canvas next to the cursor
 *  (AgentCursorLabel.tsx), so bounded here the same way `labelHint` already
 *  is: trimmed, internal whitespace (including newlines) collapsed to a
 *  single space, and capped well short of anything that would crowd the
 *  chip. */
function coercePresenceTaskLabel(value: unknown): string | null {
  if (typeof value !== 'string') return null
  const collapsed = value.trim().replace(/\s+/g, ' ')
  return collapsed ? collapsed.slice(0, 80) : null
}

/** The `POST /session/presence` payload, coerced and validated but not yet
 *  resolved against any page — resolving a targetRect/canvas position needs
 *  `resolvePresenceTargetRect` / `resolveCanvasPointForPage`, which read
 *  page/runtime state rather than just parsing the request. */
interface ParsedPresenceEvent {
  eventType: 'start' | 'surface' | 'act' | 'think' | 'done' | null
  surface: PresenceSurface | null
  activity: PresenceActivity | null
  pageId: string | null
  targetRef: string | null
  targetRefSource: PresenceTargetRefSource | null
  pageX: number | null
  pageY: number | null
  canvasX: number | null
  canvasY: number | null
  explicitTargetRect: PresenceTargetRect | null
  taskLabel: string | null
  labelHint: string | null
  labelKey: PresenceLabelKey | null
  labelParams: Record<string, string | number | boolean> | null
  targetName: string | null
  hold: boolean
}

function parsePresenceEvent(payload: Record<string, unknown>): ParsedPresenceEvent {
  const eventType =
    payload.eventType === 'start' ||
    payload.eventType === 'surface' ||
    payload.eventType === 'act' ||
    payload.eventType === 'think' ||
    payload.eventType === 'done'
      ? payload.eventType
      : null
  const coordinates =
    payload.coordinates && typeof payload.coordinates === 'object'
      ? (payload.coordinates as Record<string, unknown>)
      : {}
  const rawTargetRect = coordinates.targetRect
  const explicitTargetRect =
    rawTargetRect &&
    typeof rawTargetRect === 'object' &&
    typeof (rawTargetRect as Record<string, unknown>).x === 'number' &&
    typeof (rawTargetRect as Record<string, unknown>).y === 'number' &&
    typeof (rawTargetRect as Record<string, unknown>).width === 'number' &&
    typeof (rawTargetRect as Record<string, unknown>).height === 'number'
      ? (rawTargetRect as PresenceTargetRect)
      : null

  return {
    eventType,
    surface: coercePresenceSurface(payload.surface),
    activity: coercePresenceActivity(payload.phase),
    pageId: typeof payload.pageId === 'string' ? payload.pageId : null,
    targetRef: typeof payload.targetRef === 'string' ? payload.targetRef : null,
    targetRefSource: coercePresenceTargetRefSource(payload.targetRefSource),
    pageX: typeof coordinates.pageX === 'number' ? coordinates.pageX : null,
    pageY: typeof coordinates.pageY === 'number' ? coordinates.pageY : null,
    canvasX: typeof coordinates.canvasX === 'number' ? coordinates.canvasX : null,
    canvasY: typeof coordinates.canvasY === 'number' ? coordinates.canvasY : null,
    explicitTargetRect,
    taskLabel: coercePresenceTaskLabel(payload.taskLabel),
    labelHint: typeof payload.labelHint === 'string' ? payload.labelHint.trim().slice(0, 48) : null,
    labelKey: coercePresenceLabelKey(payload.labelKey),
    labelParams:
      payload.labelParams && typeof payload.labelParams === 'object'
        ? (payload.labelParams as Record<string, string | number | boolean>)
        : null,
    targetName: typeof payload.targetName === 'string' ? payload.targetName : null,
    // Only a `start` event can open a hold (`specular presence start`) —
    // later act/surface/think upserts for the session carry it forward via
    // `upsertActivePresenceTask`'s own stickiness, not by resending it.
    hold: eventType === 'start' && payload.hold === true,
  }
}

/** Validates the wire `queue` (a chained browse command's remaining steps —
 *  see `buildChainedPresenceSteps`), dropping any entry whose `labelKey`
 *  isn't in the presence allowlist. A dropped entry is simply skipped —
 *  `advancePendingIntent` only ever sees valid, ready-to-apply steps. */
function coercePresenceIntentQueue(value: unknown): QueuedPresenceIntent[] {
  if (!Array.isArray(value)) return []
  const queue: QueuedPresenceIntent[] = []
  for (const raw of value) {
    if (!raw || typeof raw !== 'object') continue
    const item = raw as Record<string, unknown>
    const labelKey = coercePresenceLabelKey(item.labelKey)
    const command = typeof item.command === 'string' ? item.command : null
    if (!labelKey || !command) continue
    queue.push({
      labelKey,
      command,
      targetRef: typeof item.targetRef === 'string' ? item.targetRef : null,
      targetRefSource: coercePresenceTargetRefSource(item.targetRefSource),
      targetQuery: coercePresenceTargetQuery(item.targetQuery),
      labelHint: typeof item.labelHint === 'string' ? item.labelHint.trim().slice(0, 48) : null,
    })
  }
  return queue
}

/**
 * Re-resolving targets (CSS selector / `text=` locator / `find role|testid`)
 * aren't sitting in the agent-snapshot cache the way an `@eN` ref's rect is
 * — resolving one takes a real DOM query, too slow to block the intent
 * response on. Kick it off in the background and, if it lands before this
 * intent is superseded or consumed (by the `mousePressed` that arrives via
 * the CDP proxy — see app-control-server.ts), apply the result through the
 * same `upsertPresenceCursor` / `upsertActivePresenceTask` seam every other
 * late-arriving presence update (typing, the mousePressed dwell-skip check)
 * already goes through.
 *
 * `targetQuery.selector` is already the fully-resolved CSS selector by the
 * time it reaches here — role/testid locators are translated to attribute
 * selectors at parse time (`parseTargetQuery`), so this function owns no
 * locator-kind knowledge of its own.
 */
function resolvePresenceTargetQueryInBackground(options: {
  request: IncomingMessage
  payload: Record<string, unknown>
  sessionId: string
  intentRecord: PendingIntent
  targetQuery: PresenceTargetQuery
  taskLabel: string | null
  labelHint: string | null
}): void {
  const { request, payload, sessionId, intentRecord, targetQuery, taskLabel, labelHint } = options
  // The caller only invokes this once pageId has been confirmed truthy (see
  // call site) — intentRecord.pageId is nullable because PendingIntent also
  // covers canvas-surface intents, which never reach this function.
  const pageId = intentRecord.pageId
  if (!pageId) return
  const labelKey = intentRecord.labelKey
  findPresenceTarget(pageId, {
    selector: targetQuery.selector,
    text: targetQuery.text,
    name: targetQuery.name,
    interactiveOnly: true,
  }).then((target) => {
    if (!target) return
    // Stale guard: only apply if this intent is still the one in flight.
    // A newer intent, or the mousePressed that consumes this one, deletes
    // (or replaces) the pendingIntents entry — applying a resolution that
    // lands after that would reposition the cursor for an action that's
    // already finished (or belongs to a different target).
    if (pendingIntents.get(sessionId) !== intentRecord) return
    const pagePosition = resolveCanvasPointForPage(pageId, { targetRect: target.targetRect })
    if (!pagePosition) return
    upsertActivePresenceTask(request, {
      body: payload,
      taskLabel,
      surface: 'page',
      pageId,
      canvasX: pagePosition.canvasX,
      canvasY: pagePosition.canvasY,
      targetName: target.targetName,
      targetRect: target.targetRect,
      labelHint,
    })
    upsertPresenceCursor(request, {
      body: payload,
      canvasX: pagePosition.canvasX,
      canvasY: pagePosition.canvasY,
      surface: 'page',
      activity: 'traveling',
      pageId,
      pageX: target.pageX,
      pageY: target.pageY,
      labelKey,
      taskLabel,
      labelHint,
      targetRef: target.targetRef,
      targetRefSource: target.targetRefSource,
      targetName: target.targetName,
      targetRect: target.targetRect,
    })
  }).catch(() => {})
}

interface PresenceIntentFields {
  sessionId: string
  labelKey: PresenceLabelKey
  command: string
  pageId: string | null
  targetRef: string | null
  targetRefSource: PresenceTargetRefSource | null
  targetName: string | null
  taskLabel: string | null
  labelHint: string | null
  targetQuery: PresenceTargetQuery | null
  queue: QueuedPresenceIntent[]
}

/**
 * The shared core of `/session/presence/intent`: register the pending
 * intent (so the CDP proxy can pre-position the cursor and later consume or
 * advance it) and apply the travel/task update immediately. Used both for a
 * fresh HTTP intent and — via `advancePendingIntent` — for each subsequent
 * step of a chained browse command, so a queued step gets exactly the same
 * treatment a standalone intent would.
 */
function applyPresenceIntent(
  request: IncomingMessage,
  payload: Record<string, unknown>,
  fields: PresenceIntentFields,
): void {
  const {
    sessionId, labelKey, command, pageId, targetRef, targetRefSource,
    targetName, taskLabel, labelHint, targetQuery, queue,
  } = fields

  const prev = pendingIntents.get(sessionId)
  if (prev) clearTimeout(prev.expiryTimer)
  const expiryTimer = setTimeout(
    () => advancePendingIntent(request, sessionId, payload),
    PENDING_INTENT_TTL_MS,
  )
  const intentRecord: PendingIntent = {
    labelKey,
    pageId,
    targetRef,
    targetRefSource,
    command,
    receivedAt: Date.now(),
    expiryTimer,
    queue,
    taskLabel,
  }
  pendingIntents.set(sessionId, intentRecord)

  const targetRect = resolvePresenceTargetRect(pageId, targetRef, targetRefSource, null)
  // `hover` deliberately isn't in this set: it always names a target, so
  // "no resolvable rect yet" means the resolution is still in flight
  // (see resolvePresenceTargetQueryInBackground below), not that there's no
  // target to speak of. Falling back to page center — this set's whole
  // purpose, for a targetless `wait`/bare `snapshot` — would be actively
  // misleading for a command whose entire point is pointing at something.
  const observationCommands = new Set(['snapshot', 'wait', 'get'])
  const isObservation = observationCommands.has(command)
  const currentCursor = getPresenceCursors().find((cursor) => cursor.sessionId === sessionId)
  // When we're issuing an intent for the same page the cursor is already
  // in and can't resolve a targetRect (e.g. mutation intents whose ref is
  // agent-browser-opaque, or observations like snapshot/wait/get), preserve
  // the cursor's existing position. Nulling pageX/pageY here causes
  // buildCanvasLayoutData to fall back to page center and the renderer
  // visibly hops between real interactions in the same page.
  const preserveSamePagePosition =
    pageId !== null &&
    !targetRect &&
    currentCursor?.surface === 'page' &&
    currentCursor.pageId === pageId
  const pagePosition = preserveSamePagePosition
    ? { canvasX: currentCursor.canvasX, canvasY: currentCursor.canvasY }
    : pageId && (targetRect || isObservation)
      ? resolveCanvasPointForPage(pageId, { targetRect })
      : null

  upsertActivePresenceTask(request, {
    body: payload,
    taskLabel,
    surface: pageId ? 'page' : 'canvas',
    pageId,
    canvasX: pagePosition?.canvasX,
    canvasY: pagePosition?.canvasY,
    targetName,
    targetRect,
    labelHint,
  })

  upsertPresenceCursor(request, {
    body: payload,
    canvasX: pagePosition?.canvasX,
    canvasY: pagePosition?.canvasY,
    surface: pageId ? 'page' : 'canvas',
    activity: 'traveling',
    pageId,
    pageX: preserveSamePagePosition ? undefined : null,
    pageY: preserveSamePagePosition ? undefined : null,
    labelKey,
    taskLabel,
    labelHint,
    targetRef,
    targetRefSource,
    targetName,
    targetRect,
  })

  // pageId is narrowed to string (not null) here — the query is only
  // meaningful once we know which page to resolve it against.
  if (pageId && targetQuery && !targetRef) {
    resolvePresenceTargetQueryInBackground({
      request,
      payload,
      sessionId,
      intentRecord,
      targetQuery,
      taskLabel,
      labelHint,
    })
  } else if (pageId && targetRef && targetRefSource === 'agent-browser') {
    // The ref itself is opaque (resolvePresenceTargetRect returns null for
    // it above), but a prior `snapshot` may have told main its role/name —
    // synthesize the same re-resolving query a selector/text target would
    // carry and run it through the identical background path, stale guard
    // included. Only fires when that's safe (see synthesizeAgentBrowserTargetQuery).
    const synthesized = synthesizeAgentBrowserTargetQuery(
      sessionId,
      pageId,
      targetRef.replace(/^@/, ''),
    )
    if (synthesized) {
      resolvePresenceTargetQueryInBackground({
        request,
        payload,
        sessionId,
        intentRecord,
        targetQuery: synthesized,
        taskLabel,
        labelHint,
      })
    }
  }

  scheduleThinkingState(request)
}

/**
 * Consumes the current pending intent — a real CDP event (`mousePressed`,
 * app-control-server.ts) or its own TTL — and advances to the next queued
 * step of a chained browse command, if any, giving it the exact same
 * travel/task application a fresh intent gets. Deletes the pending intent
 * outright when the queue is empty, matching the pre-chaining behavior for
 * a single (unqueued) intent.
 *
 * Advancing only on a real consumption signal (never speculatively) is what
 * keeps a queue from racing ahead of the chain it describes; the per-step
 * TTL is the backstop for steps with no such signal (fill/type without a
 * preceding click, wait, get, select) — see buildChainedPresenceSteps.
 */
export function advancePendingIntent(
  request: IncomingMessage,
  sessionId: string,
  body: Record<string, unknown>,
): void {
  const current = pendingIntents.get(sessionId)
  if (!current) return
  clearTimeout(current.expiryTimer)
  pendingIntents.delete(sessionId)
  const [next, ...rest] = current.queue
  if (!next) return
  applyPresenceIntent(request, body, {
    sessionId,
    labelKey: next.labelKey,
    command: next.command,
    pageId: current.pageId,
    targetRef: next.targetRef,
    targetRefSource: next.targetRefSource,
    targetName: null,
    taskLabel: current.taskLabel,
    labelHint: next.labelHint,
    targetQuery: next.targetQuery,
    queue: rest,
  })
}

export const sessionRoutes: Route[] = [
  {
    method: 'GET',
    pattern: '/session/presence',
    async handler({ response }) {
      writeJson(response, 200, { cursors: getPresenceCursors() })
    },
  },
  {
    method: 'POST',
    pattern: '/session/presence',
    async handler({ request, response, body }) {
      const payload = body as Record<string, unknown>
      const event = parsePresenceEvent(payload)

      if (event.eventType === 'done') {
        clearActivePresenceTask(request, payload)
        writeJson(response, 200, { ok: true })
        return
      }
      if (!event.surface || !event.activity) {
        writeJson(response, 400, { error: 'surface and phase are required' })
        return
      }

      const targetRect = resolvePresenceTargetRect(
        event.pageId,
        event.targetRef,
        event.targetRefSource,
        event.explicitTargetRect,
      )
      const pagePosition =
        event.surface === 'page' && event.pageId
          ? resolveCanvasPointForPage(event.pageId, { pageX: event.pageX, pageY: event.pageY, targetRect })
          : null

      if (['start', 'surface', 'act', 'think'].includes(event.eventType ?? '')) {
        upsertActivePresenceTask(request, {
          body: payload,
          taskLabel: event.taskLabel,
          surface: event.surface,
          pageId: event.pageId,
          pageX: event.pageX,
          pageY: event.pageY,
          canvasX: event.canvasX ?? pagePosition?.canvasX ?? null,
          canvasY: event.canvasY ?? pagePosition?.canvasY ?? null,
          targetName: event.targetName,
          targetRect,
          labelHint: event.labelHint,
          hold: event.hold,
        })
      }
      upsertPresenceCursor(request, {
        body: payload,
        canvasX: event.canvasX ?? pagePosition?.canvasX,
        canvasY: event.canvasY ?? pagePosition?.canvasY,
        surface: event.surface,
        activity: event.eventType === 'think' ? 'thinking' : event.activity,
        pageId: event.pageId,
        pageX: event.pageX,
        pageY: event.pageY,
        labelKey: event.eventType === 'think' ? 'thinking' : event.labelKey,
        taskLabel: event.taskLabel,
        labelHint: event.labelHint,
        labelParams: event.labelParams,
        targetRef: event.targetRef,
        targetRefSource: event.targetRefSource,
        targetName: event.targetName,
        targetRect,
      })
      if (
        event.pageId &&
        event.targetRef &&
        event.labelKey &&
        ['click_target', 'type_text', 'wait_page'].includes(event.labelKey)
      ) {
        invalidateAgentSnapshot(event.pageId)
      }
      scheduleThinkingState(request)
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/session/presence/intent',
    async handler({ request, response, body }) {
      const payload = body as Record<string, unknown>
      const resolved = resolveSession(request, payload)
      if (!resolved) {
        writeJson(response, 400, { error: 'session required' })
        return
      }
      const labelKey = coercePresenceLabelKey(payload.labelKey)
      const command = typeof payload.command === 'string' ? payload.command : null
      let pageId = typeof payload.pageId === 'string' ? payload.pageId : null
      if (!pageId) {
        for (const reg of cdpProxyRegistrations.values()) {
          if (reg.sessionId === resolved.sessionId) {
            pageId = reg.pageId
            break
          }
        }
      }
      const targetRef = typeof payload.targetRef === 'string' ? payload.targetRef : null
      const targetRefSource = coercePresenceTargetRefSource(payload.targetRefSource)
      const targetName = typeof payload.targetName === 'string' ? payload.targetName : null
      const taskLabel = coercePresenceTaskLabel(payload.taskLabel)
      const labelHint = typeof payload.labelHint === 'string' ? payload.labelHint.trim().slice(0, 48) : null

      if (!labelKey || !command) {
        writeJson(response, 400, { error: 'labelKey and command are required' })
        return
      }

      const targetQuery = coercePresenceTargetQuery(payload.targetQuery)
      // A fresh intent always replaces whatever queue the session had in
      // flight — a chain never partially survives a new call.
      const queue = coercePresenceIntentQueue(payload.queue)

      applyPresenceIntent(request, payload, {
        sessionId: resolved.sessionId,
        labelKey,
        command,
        pageId,
        targetRef,
        targetRefSource,
        targetName,
        taskLabel,
        labelHint,
        targetQuery,
        queue,
      })

      writeJson(response, 200, { ok: true })
    },
  },
  {
    // Fire-and-forget from handleBrowse after every successful `snapshot` —
    // replaces the session+page's agent-browser ref map wholesale so a
    // later `/session/presence/intent` for an `@eN` ref can synthesize a
    // targetQuery from it (see synthesizeAgentBrowserTargetQuery). Never
    // slows or fails the snapshot command that triggered it: this handler
    // does no async work and always answers 200, even for a malformed body.
    method: 'POST',
    pattern: '/session/presence/agent-browser-refs',
    async handler({ request, response, body }) {
      const payload = body as Record<string, unknown>
      const resolved = resolveSession(request, payload)
      const pageId = typeof payload.pageId === 'string' ? payload.pageId : null
      const rawRefs = Array.isArray(payload.refs) ? payload.refs : []
      if (resolved && pageId) {
        const refs: Array<{ ref: string; role: string; name: string | null }> = []
        for (const raw of rawRefs) {
          if (!raw || typeof raw !== 'object') continue
          const item = raw as Record<string, unknown>
          if (typeof item.ref !== 'string' || typeof item.role !== 'string') continue
          refs.push({
            ref: item.ref,
            role: item.role,
            name: typeof item.name === 'string' ? item.name : null,
          })
        }
        setAgentBrowserRefs(resolved.sessionId, pageId, refs)
      }
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/mcp/session/open',
    async handler({ response, body }) {
      const payload = body as { sessionId?: string; clientName?: string }
      if (!payload.sessionId) {
        writeJson(response, 400, { error: 'sessionId is required' })
        return
      }
      mcpSessions.set(payload.sessionId, {
        id: payload.sessionId,
        clientName: payload.clientName ?? 'specular-mcp',
        lastSeenAt: Date.now(),
      })
      notifyStatusListeners()
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/mcp/session/ping',
    async handler({ response, body }) {
      const payload = body as { sessionId?: string; clientName?: string }
      if (!payload.sessionId) {
        writeJson(response, 400, { error: 'sessionId is required' })
        return
      }
      const existing = mcpSessions.get(payload.sessionId)
      mcpSessions.set(payload.sessionId, {
        id: payload.sessionId,
        clientName: payload.clientName ?? existing?.clientName ?? 'specular-mcp',
        lastSeenAt: Date.now(),
      })
      notifyStatusListeners()
      writeJson(response, 200, { ok: true })
    },
  },
  {
    method: 'POST',
    pattern: '/mcp/session/close',
    async handler({ response, body }) {
      const payload = body as { sessionId?: string }
      if (!payload.sessionId) {
        writeJson(response, 400, { error: 'sessionId is required' })
        return
      }
      mcpSessions.delete(payload.sessionId)
      notifyStatusListeners()
      beginPresenceDeparture(payload.sessionId)
      writeJson(response, 200, { ok: true })
    },
  },
]
