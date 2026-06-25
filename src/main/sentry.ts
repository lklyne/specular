import * as Sentry from '@sentry/electron/main'
import type { ErrorEvent } from '@sentry/core'
import { app } from 'electron'
import {
  markSentryEnabled,
  setAutoUpdateChannel,
} from './sentry-context'
import { workspaceGroups, workspaceTabs } from './runtime/workspace-model'
import { pages } from './runtime/page-runtime'
import { textEntities } from './runtime/text-entity-state'
import { fileEntities } from './runtime/file-entity-state'
import { drawingEntities } from './runtime/drawing-entity-state'
import { shapeEntities } from './runtime/shape-entity-state'

/**
 * Initialize Sentry for the Electron main process.
 *
 * Sentry is a no-op when `SENTRY_DSN` is unset (the default), so this is
 * safe to call unconditionally in development. Renderer processes should
 * call `initRendererSentry` from `renderer/shared/sentry-init.ts`; they
 * inherit config from the main process via IPC and do not need a DSN.
 *
 * Error-only config — no tracing, no session replay.
 *
 * Privacy: users' canvases contain their own URLs and text content, so we
 * scrub aggressively. URLs are reduced to `scheme://host`, user/cookies/
 * request bodies are dropped, and any URL-shaped text in messages or
 * breadcrumb data is redacted the same way.
 */
export function initSentry(): void {
  // Prefer the build-time injected DSN (see vite.main.config.ts `define`) so
  // distributed .dmg builds self-configure without a runtime env var. Fall
  // back to process.env for local dev launched outside Vite (e.g. raw `node`
  // scripts) — pnpm dev goes through Vite, which bakes the env in.
  const dsn =
    (import.meta as unknown as { env?: { SENTRY_DSN?: string } }).env?.SENTRY_DSN ||
    process.env.SENTRY_DSN
  if (!dsn) {
    return
  }

  Sentry.init({
    dsn,
    release: `specular@${app.getVersion()}`,
    environment: process.env.NODE_ENV === 'development' ? 'development' : 'production',
    sendDefaultPii: false,
    tracesSampleRate: 0,
    beforeSend: enrichAndScrub,
    beforeBreadcrumb: scrubBreadcrumb,
  })

  markSentryEnabled()
  setAutoUpdateChannel(app.getVersion())
}

/**
 * Live app-state tags — read fresh on every event so entity counts and
 * entity counts reflect the moment of capture without any subscription
 * plumbing on the mutation side.
 */
function readAppStateTags(): Record<string, string> {
  return {
    tab_count: String(workspaceTabs.length),
    page_count: String(pages.length),
    canvas_entity_count: String(
      textEntities.length +
        fileEntities.length +
        drawingEntities.length +
        shapeEntities.length +
        workspaceGroups.length,
    ),
  }
}

function enrichAndScrub(event: ErrorEvent): ErrorEvent | null {
  event.tags = { ...readAppStateTags(), ...event.tags }
  return scrubEvent(event)
}

function scrubEvent(event: ErrorEvent): ErrorEvent | null {
  if (event.request) {
    delete event.request.url
    delete event.request.cookies
    delete event.request.headers
    delete event.request.data
    delete event.request.query_string
  }
  if (event.user) {
    // Preserve anonymous install id; drop everything else that might be PII.
    event.user = event.user.id ? { id: event.user.id } : undefined
  }

  if (event.message) {
    event.message = redactUrls(event.message)
  }

  if (event.exception?.values) {
    for (const ex of event.exception.values) {
      if (ex.value) ex.value = redactUrls(ex.value)
    }
  }

  if (event.extra) {
    event.extra = scrubRecord(event.extra)
  }
  if (event.contexts) {
    for (const key of Object.keys(event.contexts)) {
      const ctx = event.contexts[key]
      if (ctx && typeof ctx === 'object') {
        event.contexts[key] = scrubRecord(ctx as Record<string, unknown>) as typeof ctx
      }
    }
  }

  return event
}

function scrubBreadcrumb(breadcrumb: Sentry.Breadcrumb): Sentry.Breadcrumb | null {
  if (breadcrumb.data && typeof breadcrumb.data === 'object') {
    breadcrumb.data = scrubRecord(breadcrumb.data)
  }
  if (breadcrumb.message) {
    breadcrumb.message = redactUrls(breadcrumb.message)
  }
  return breadcrumb
}

function scrubRecord(input: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(input)) {
    if (typeof v === 'string') {
      out[k] = isLikelyUrl(k, v) ? redactUrl(v) : redactUrls(v)
    } else {
      out[k] = v
    }
  }
  return out
}

function isLikelyUrl(key: string, value: string): boolean {
  return /url|href|uri|src/i.test(key) && /^https?:\/\//i.test(value)
}

function redactUrl(url: string): string {
  try {
    const u = new URL(url)
    return `${u.protocol}//${u.host}`
  } catch {
    return url
  }
}

function redactUrls(text: string): string {
  return text.replace(/https?:\/\/[^\s'"<>`)]+/g, (m) => redactUrl(m))
}
