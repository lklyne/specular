import type { Route } from './types'
import type { PresenceTargetRect, PresenceTargetRefSource } from '../../shared/types'
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
} from '../presence-manager'
import { mcpSessions, resolveSession } from '../presence-session'
import { invalidateAgentSnapshot } from '../runtime/agent-snapshot-cache'
import { cdpProxyRegistrations } from '../cdp-proxy'
import { writeJson, notifyStatusListeners } from './http-helpers'

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

      const prev = pendingIntents.get(resolved.sessionId)
      if (prev) clearTimeout(prev.expiryTimer)
      const expiryTimer = setTimeout(() => pendingIntents.delete(resolved.sessionId), PENDING_INTENT_TTL_MS)
      pendingIntents.set(resolved.sessionId, {
        labelKey,
        pageId,
        targetRef,
        targetRefSource,
        command,
        receivedAt: Date.now(),
        expiryTimer,
      })

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
