import { nativeImage } from 'electron'
import type { Route } from './types'
import { focusTargets } from '../workspace-groups'
import { createPageAtPosition } from '../runtime/document-commands'
import { findPageById } from '../runtime/runtime-context'
import {
  takePageAgentSnapshot,
  takePageScreenshot,
  takePageSnapshot,
  queryPageElements,
} from '../runtime/page-runtime'
import { cacheAgentSnapshot } from '../runtime/agent-snapshot-cache'
import { captureFrameComposited } from '../runtime/frame-compositor'
import { win } from '../runtime/window-shell'
import {
  resolvePageCdpConnection,
  registerPageCdpProxy,
  cdpProxyRegistrations,
  pruneExpiredCdpProxyRegistrations,
  summarizeCdpProxyRegistration,
  cdpProxyMetrics,
} from '../cdp-proxy'
import { movePresenceCursorTo } from '../presence-cursor'
import {
  normalizeAgentSnapshot,
  findPresenceTarget,
} from '../presence-manager'
import { resolveSession } from '../presence-session'
import { writeJson, getServerAddress } from './http-helpers'

export const pageRoutes: Route[] = [
  {
    method: 'GET',
    pattern: /^\/pages\/([^/]+)\/cdp-target$/,
    async handler({ request, response, params }) {
      try {
        const pageId = decodeURIComponent(params[0])
        const connection = await resolvePageCdpConnection(pageId)
        const address = getServerAddress()
        if (!address || typeof address === 'string') {
          throw new Error('CDP proxy server is unavailable')
        }
        const resolved = resolveSession(request)
        const registration = registerPageCdpProxy(connection, address.port, {
          sessionId: resolved?.sessionId ?? null,
          clientName: resolved?.session.clientName ?? null,
        })
        // D8 (issue #318): expose the page's navigation generation and the
        // baseline recorded at snapshot time so browse-handler.ts can warn
        // agents when refs from an older snapshot are used after the page
        // has navigated.
        const page = findPageById(pageId)
        writeJson(response, 200, {
          ...registration,
          generation: page?.navGeneration ?? 0,
          lastSnapshotGeneration: page?.lastAgentSnapshotGeneration ?? null,
        })
      } catch (error) {
        const message = error instanceof Error ? error.message : 'Unable to resolve CDP target'
        const status =
          message === 'Page not found'
            ? 404
            : message === 'CDP target not found for page' || message === 'CDP browser target not found'
              ? 404
            : 502
        writeJson(response, status, { error: message })
      }
    },
  },
  {
    // D8 (issue #318): marks that an agent snapshot of this page just
    // completed. The baseline lives on the main-process Page object because
    // the `specular` CLI runs one fresh process per command — CLI-side state
    // can't survive the snapshot→mutate loop the staleness check spans. The
    // route stamps the page's own current navGeneration rather than trusting
    // a client-supplied number: a long-lived client (the MCP server) reads
    // generations through a 60s cache, and a stale-low baseline would fire
    // false staleness warnings on fresh snapshots.
    method: 'POST',
    pattern: /^\/pages\/([^/]+)\/snapshot-seen$/,
    async handler({ response, params }) {
      const pageId = decodeURIComponent(params[0])
      const page = findPageById(pageId)
      if (!page) {
        writeJson(response, 404, { error: `Page not found: ${pageId}` })
        return
      }
      page.lastAgentSnapshotGeneration = page.navGeneration
      writeJson(response, 200, { ok: true, generation: page.navGeneration })
    },
  },
  {
    method: 'GET',
    pattern: '/debug/cdp-proxy',
    async handler({ response }) {
      pruneExpiredCdpProxyRegistrations()
      writeJson(response, 200, {
        registrations: [...cdpProxyRegistrations.values()].map(summarizeCdpProxyRegistration),
        metrics: cdpProxyMetrics,
      })
    },
  },
  {
    method: 'POST',
    pattern: '/pages/create-at-position',
    async handler({ request, response, body }) {
      const payload = body as {
        sourcePageId?: string
        presetIndex?: number
        canvasX?: number
        canvasY?: number
      }
      if (typeof payload.canvasX !== 'number' || typeof payload.canvasY !== 'number') {
        writeJson(response, 400, { error: 'canvasX and canvasY are required numbers' })
        return
      }
      movePresenceCursorTo(request, payload.canvasX, payload.canvasY, 'create_page')
      writeJson(
        response,
        200,
        createPageAtPosition({
          sourcePageId: payload.sourcePageId,
          presetIndex: payload.presetIndex ?? 0,
          canvasX: payload.canvasX,
          canvasY: payload.canvasY,
          mode: 'add_from_toolbar',
          focus: true,
        }),
      )
    },
  },
  {
    method: 'POST',
    pattern: '/pages/snapshot',
    async handler({ response, body }) {
      const payload = body as { pageId?: string; maxDepth?: number }
      const snapshot = await takePageSnapshot(payload.pageId, payload.maxDepth)
      writeJson(response, 200, { snapshot })
    },
  },
  {
    method: 'POST',
    pattern: '/pages/agent-snapshot',
    async handler({ response, body }) {
      const payload = body as { pageId?: string; maxDepth?: number }
      const pageId = typeof payload.pageId === 'string' ? payload.pageId : undefined
      if (!pageId) {
        writeJson(response, 400, { error: 'pageId is required' })
        return
      }
      const rawSnapshot = await takePageAgentSnapshot(pageId, payload.maxDepth)
      const snapshot = normalizeAgentSnapshot(pageId, rawSnapshot)
      cacheAgentSnapshot(snapshot)
      writeJson(response, 200, { snapshot })
    },
  },
  {
    method: 'POST',
    pattern: '/pages/screenshot',
    async handler({ response, body }) {
      const payload = body as { pageId?: string }
      const base64 = await takePageScreenshot(payload.pageId)
      writeJson(response, 200, { base64, mimeType: 'image/png' })
    },
  },
  {
    method: 'POST',
    pattern: '/pages/screenshot-composite',
    async handler({ response, body }) {
      const payload = body as { pageId?: string; padding?: number }
      const pageId = payload.pageId
      if (!pageId) {
        writeJson(response, 400, { error: 'pageId is required' })
        return
      }
      const page = findPageById(pageId)
      if (!page) {
        writeJson(response, 404, { error: `Page not found: ${pageId}` })
        return
      }
      if (!win || win.isDestroyed()) {
        writeJson(response, 500, { error: 'Window not available' })
        return
      }
      try {
        focusTargets({ pageIds: [pageId] })
        await new Promise((r) => setTimeout(r, 400))

        const result = await captureFrameComposited(page)
        if (!result) {
          writeJson(response, 500, { error: 'Page capture failed (destroyed or empty)' })
          return
        }

        const composited = nativeImage.createFromBitmap(result.bitmap, { width: result.width, height: result.height })
        const base64 = composited.toPNG().toString('base64')
        writeJson(response, 200, { base64, mimeType: 'image/png', width: result.width, height: result.height })
      } catch (error) {
        writeJson(response, 500, { error: error instanceof Error ? error.message : 'Screenshot failed' })
      }
    },
  },
  {
    method: 'POST',
    pattern: '/pages/query-elements',
    async handler({ response, body }) {
      const payload = body as { pageId?: string; selector?: string; maxResults?: number }
      const elements = await queryPageElements(payload.pageId, payload.selector, payload.maxResults)
      writeJson(response, 200, { elements })
    },
  },
  {
    method: 'POST',
    pattern: '/pages/find-target',
    async handler({ response, body }) {
      const payload = body as {
        pageId?: string
        selector?: string
        name?: string
        text?: string
        elementPath?: string
        fullPath?: string
        interactiveOnly?: boolean
        maxResults?: number
      }
      if (!payload.pageId) {
        writeJson(response, 400, { error: 'pageId is required' })
        return
      }
      const target = await findPresenceTarget(payload.pageId, {
        selector: typeof payload.selector === 'string' ? payload.selector : null,
        name: typeof payload.name === 'string' ? payload.name : null,
        text: typeof payload.text === 'string' ? payload.text : null,
        elementPath: typeof payload.elementPath === 'string' ? payload.elementPath : null,
        fullPath: typeof payload.fullPath === 'string' ? payload.fullPath : null,
        interactiveOnly: payload.interactiveOnly !== false,
        maxResults: typeof payload.maxResults === 'number' ? payload.maxResults : undefined,
      })
      if (!target) {
        writeJson(response, 404, { error: 'No matching target found' })
        return
      }
      writeJson(response, 200, { target })
    },
  },
]
