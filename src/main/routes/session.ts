import type { IncomingMessage } from 'http'
import type { Route } from './types'
import type { PresenceLabelKey, PresenceTargetQuery, PresenceTargetRect, PresenceTargetRefSource } from '../../shared/types'
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
  type PendingIntent,
} from '../presence-manager'
import { mcpSessions, resolveSession } from '../presence-session'
import { invalidateAgentSnapshot } from '../runtime/agent-snapshot-cache'
import { cdpProxyRegistrations } from '../cdp-proxy'
import { writeJson, notifyStatusListeners } from './http-helpers'

function coercePresenceTargetQuery(value: unknown): PresenceTargetQuery | null {
  if (!value || typeof value !== 'object') return null
  const payload = value as Record<string, unknown>
  const selector = typeof payload.selector === 'string' ? payload.selector : null
  const text = typeof payload.text === 'string' ? payload.text : null
  const role = typeof payload.role === 'string' ? payload.role : null
  const name = typeof payload.name === 'string' ? payload.name : null
  if (!selector && !text && !role) return null
  return { selector, text, role, name }
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
 */
function resolvePresenceTargetQueryInBackground(
  request: IncomingMessage,
  payload: Record<string, unknown>,
  sessionId: string,
  pageId: string,
  intentRecord: PendingIntent,
  targetQuery: PresenceTargetQuery,
  labelKey: PresenceLabelKey,
  taskLabel: string | null,
  labelHint: string | null,
): void {
  const selector = targetQuery.selector ?? (targetQuery.role ? `[role="${targetQuery.role}"]` : null)
  findPresenceTarget(pageId, {
    selector,
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
      const eventType =
        payload.eventType === 'start' ||
        payload.eventType === 'surface' ||
        payload.eventType === 'act' ||
        payload.eventType === 'think' ||
        payload.eventType === 'done'
          ? payload.eventType
          : null
      const surface = coercePresenceSurface(payload.surface)
      const activity = coercePresenceActivity(payload.phase)
      if (eventType === 'done') {
        clearActivePresenceTask(request, payload)
        writeJson(response, 200, { ok: true })
        return
      }
      if (!surface || !activity) {
        writeJson(response, 400, { error: 'surface and phase are required' })
        return
      }
      const coordinates =
        payload.coordinates && typeof payload.coordinates === 'object'
          ? (payload.coordinates as Record<string, unknown>)
          : {}
      const pageId = typeof payload.pageId === 'string' ? payload.pageId : null
      const targetRef = typeof payload.targetRef === 'string' ? payload.targetRef : null
      const targetRefSource = coercePresenceTargetRefSource(payload.targetRefSource)
      const pageX = typeof coordinates.pageX === 'number' ? coordinates.pageX : null
      const pageY = typeof coordinates.pageY === 'number' ? coordinates.pageY : null
      const explicitTargetRect =
        coordinates.targetRect &&
        typeof coordinates.targetRect === 'object' &&
        typeof (coordinates.targetRect as Record<string, unknown>).x === 'number' &&
        typeof (coordinates.targetRect as Record<string, unknown>).y === 'number' &&
        typeof (coordinates.targetRect as Record<string, unknown>).width === 'number' &&
        typeof (coordinates.targetRect as Record<string, unknown>).height === 'number'
          ? (coordinates.targetRect as PresenceTargetRect)
          : null
      const targetRect = resolvePresenceTargetRect(pageId, targetRef, targetRefSource, explicitTargetRect)
      const pagePosition =
        surface === 'page' && pageId
          ? resolveCanvasPointForPage(pageId, { pageX, pageY, targetRect })
          : null
      const taskLabel = typeof payload.taskLabel === 'string' ? payload.taskLabel : null
      const labelHint = typeof payload.labelHint === 'string' ? payload.labelHint.trim().slice(0, 48) : null
      if (eventType === 'start' || eventType === 'surface' || eventType === 'act' || eventType === 'think') {
        upsertActivePresenceTask(request, {
          body: payload,
          taskLabel,
          surface,
          pageId,
          pageX,
          pageY,
          canvasX:
            typeof coordinates.canvasX === 'number'
              ? coordinates.canvasX
              : pagePosition?.canvasX ?? null,
          canvasY:
            typeof coordinates.canvasY === 'number'
              ? coordinates.canvasY
              : pagePosition?.canvasY ?? null,
          targetName: typeof payload.targetName === 'string' ? payload.targetName : null,
          targetRect,
          labelHint,
        })
      }
      upsertPresenceCursor(request, {
        body: payload,
        canvasX:
          typeof coordinates.canvasX === 'number'
            ? coordinates.canvasX
            : pagePosition?.canvasX,
        canvasY:
          typeof coordinates.canvasY === 'number'
            ? coordinates.canvasY
            : pagePosition?.canvasY,
        surface,
        activity: eventType === 'think' ? 'thinking' : activity,
        pageId,
        pageX,
        pageY,
        labelKey:
          eventType === 'think'
            ? 'thinking'
            : coercePresenceLabelKey(payload.labelKey),
        taskLabel,
        labelHint,
        labelParams:
          payload.labelParams && typeof payload.labelParams === 'object'
            ? (payload.labelParams as Record<string, string | number | boolean>)
            : null,
        targetRef,
        targetRefSource,
        targetName: typeof payload.targetName === 'string' ? payload.targetName : null,
        targetRect,
      })
      if (pageId && targetRef && ['click_target', 'type_text', 'wait_page'].includes(String(payload.labelKey))) {
        invalidateAgentSnapshot(pageId)
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
      const taskLabel = typeof payload.taskLabel === 'string' ? payload.taskLabel : null
      const labelHint = typeof payload.labelHint === 'string' ? payload.labelHint.trim().slice(0, 48) : null

      if (!labelKey || !command) {
        writeJson(response, 400, { error: 'labelKey and command are required' })
        return
      }

      const targetQuery = coercePresenceTargetQuery(payload.targetQuery)

      const prev = pendingIntents.get(resolved.sessionId)
      if (prev) clearTimeout(prev.expiryTimer)
      const expiryTimer = setTimeout(() => pendingIntents.delete(resolved.sessionId), PENDING_INTENT_TTL_MS)
      const intentRecord: PendingIntent = {
        labelKey,
        pageId,
        targetRef,
        targetRefSource,
        command,
        receivedAt: Date.now(),
        expiryTimer,
      }
      pendingIntents.set(resolved.sessionId, intentRecord)

      const targetRect = resolvePresenceTargetRect(pageId, targetRef, targetRefSource, null)
      const observationCommands = new Set(['snapshot', 'wait', 'get'])
      const isObservation = observationCommands.has(command)
      const currentCursor = getPresenceCursors().find(
        (cursor) => cursor.sessionId === resolved.sessionId,
      )
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
        ? {
            canvasX: currentCursor.canvasX,
            canvasY: currentCursor.canvasY,
          }
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
        resolvePresenceTargetQueryInBackground(
          request,
          payload,
          resolved.sessionId,
          pageId,
          intentRecord,
          targetQuery,
          labelKey,
          taskLabel,
          labelHint,
        )
      }

      scheduleThinkingState(request)
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
