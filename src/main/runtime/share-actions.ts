/**
 * Cloud-share actions — the main-process side of the share popover (ADR 0018
 * §4b, cloud-sync spike step 7). These are the functions the toolbar IPC
 * handlers call; the popover renders entirely from `shareState()` and drives
 * publish / copy-link / reset / revoke through the rest.
 *
 * Design invariants:
 *   - Every exported action returns a typed `ShareResult` (or a plain
 *     `ShareStateData`); network failures never throw across the IPC boundary.
 *   - The owner principal is an anonymous better-auth session persisted by
 *     `cloud-credentials` (tier 1), reused across calls — never re-minted.
 *   - "Copy link" is the publish moment (ADR 0018 §4b): the first copy
 *     auto-publishes, attaches the transport, mints the grant, and writes the
 *     clipboard main-side so the renderer never handles the token.
 *
 * Node's global `fetch` is used directly (main is Node 22).
 */

import { clipboard } from 'electron'
import type {
  ShareLinkInfo,
  ShareResult,
  ShareScope,
  ShareStateData,
} from '../../shared/share'
import { allEntities } from '../entities/contract'
import { buildShareLink, parseShareLink, redeemLink } from '../sync-client/share-link'
import {
  getStoredSession,
  storeSession,
  type DeviceSession,
} from './cloud-credentials'
import { getCloudShareConfig } from './preferences'
import { getSyncBinding, getSyncStatus, publishBinding, setSyncStatus } from './workspace-sync'
import { connectSyncTransport, getSyncProvider } from './workspace-sync-transport'

const SCOPES: readonly ShareScope[] = ['view', 'comment', 'edit']

function isScope(value: unknown): value is ShareScope {
  return typeof value === 'string' && (SCOPES as readonly string[]).includes(value)
}

function ownerHeaders(cookie: string): Record<string, string> {
  return { 'content-type': 'application/json', cookie }
}

/**
 * No entities anywhere in the workspace — the precondition for a safe join.
 * Reads the runtime stores, not the Y.Doc: forward sync runs on a microtask
 * after `scheduleWorkspaceAutosave()`, so a just-created entity is in the
 * runtime and not yet in the doc.
 */
function isWorkspaceEmpty(): boolean {
  return allEntities().length === 0
}

// ---------------------------------------------------------------------------
// Device session (anonymous owner principal)
// ---------------------------------------------------------------------------

/**
 * Reuse the stored owner session for `serverUrl`, or mint one on first contact:
 * POST the anonymous sign-in, capture the session cookie, persist it. The
 * cookie is the credential every owner-scoped call presents.
 */
async function ensureDeviceSession(serverUrl: string): Promise<DeviceSession> {
  const existing = getStoredSession(serverUrl)
  if (existing) return existing

  const res = await fetch(`${serverUrl}/api/auth/sign-in/anonymous`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: '{}',
  })
  if (!res.ok) throw new Error(`anonymous sign-in failed (${res.status})`)

  const body = (await res.json()) as { user?: { id?: string } }
  const cookie = res.headers
    .getSetCookie()
    .map((c) => c.split(';')[0])
    .join('; ')
  if (!cookie) throw new Error('anonymous sign-in returned no session cookie')

  const session: DeviceSession = { userId: body.user?.id ?? '', cookie }
  storeSession(serverUrl, session)
  return session
}

// ---------------------------------------------------------------------------
// Publish / transport
// ---------------------------------------------------------------------------

async function ownerConnectToken(
  serverUrl: string,
  cookie: string,
  docId: string,
): Promise<string> {
  const res = await fetch(`${serverUrl}/docs/${docId}/connect`, {
    method: 'POST',
    headers: ownerHeaders(cookie),
  })
  if (!res.ok) throw new Error(`owner connect failed (${res.status})`)
  const body = (await res.json()) as { token?: string }
  if (!body.token) throw new Error('owner connect returned no token')
  return body.token
}

/**
 * Ensure the workspace is published and the transport is attached. Idempotent:
 * an already-bound workspace only (re)connects the transport if it dropped.
 */
async function ensurePublished(): Promise<void> {
  const { serverUrl } = getCloudShareConfig()
  const session = await ensureDeviceSession(serverUrl)

  let binding = getSyncBinding()
  if (!binding) {
    const res = await fetch(`${serverUrl}/docs`, {
      method: 'POST',
      headers: ownerHeaders(session.cookie),
    })
    if (!res.ok) throw new Error(`publish (create doc) failed (${res.status})`)
    const body = (await res.json()) as { docId?: string }
    if (!body.docId) throw new Error('create doc returned no docId')
    publishBinding({ docId: body.docId, url: serverUrl })
    binding = getSyncBinding()
  }
  if (!binding) throw new Error('publish did not establish a binding')

  if (!getSyncProvider()) {
    const token = await ownerConnectToken(serverUrl, session.cookie, binding.docId)
    connectSyncTransport(token)
  }
}

// ---------------------------------------------------------------------------
// Link operations
// ---------------------------------------------------------------------------

interface ServerLink {
  grantId: string
  scope: string
  token: string
}

function toLinkInfo(serverUrl: string, docId: string, link: ServerLink): ShareLinkInfo {
  return {
    grantId: link.grantId,
    scope: isScope(link.scope) ? link.scope : 'comment',
    url: buildShareLink({ base: serverUrl, docId, token: link.token }),
  }
}

async function fetchLinks(
  serverUrl: string,
  cookie: string,
  docId: string,
): Promise<ShareLinkInfo[]> {
  const res = await fetch(`${serverUrl}/docs/${docId}/links`, {
    headers: ownerHeaders(cookie),
  })
  if (!res.ok) throw new Error(`list links failed (${res.status})`)
  const body = (await res.json()) as { links?: ServerLink[] }
  return (body.links ?? []).map((l) => toLinkInfo(serverUrl, docId, l))
}

// ---------------------------------------------------------------------------
// Exported actions — each returns typed data, never throws across IPC
// ---------------------------------------------------------------------------

/**
 * Wrap an action so any failure becomes `{ ok: false, error }`. A published
 * workspace whose server is unreachable is marked 'error' so the indicator can
 * reflect it; an unpublished one has no rendezvous and stays 'off'.
 */
async function guard<T>(fn: () => Promise<T>): Promise<ShareResult<T>> {
  try {
    return { ok: true, value: await fn() }
  } catch (error) {
    if (getSyncBinding()) setSyncStatus('error')
    return { ok: false, error: error instanceof Error ? error.message : String(error) }
  }
}

/** Snapshot the popover renders from. Best-effort link list; degrades cleanly. */
export async function shareState(): Promise<ShareStateData> {
  const { enabled, serverUrl } = getCloudShareConfig()
  const binding = getSyncBinding()
  const base: ShareStateData = { enabled, serverUrl, binding, status: getSyncStatus() }
  if (!enabled || !binding) return base

  const session = getStoredSession(serverUrl)
  if (!session) return base
  try {
    const links = await fetchLinks(serverUrl, session.cookie, binding.docId)
    return { ...base, links }
  } catch {
    return { ...base, status: 'error' }
  }
}

/** Publish the workspace (first-copy moment) and attach the transport. */
export async function sharePublish(): Promise<ShareResult<ShareStateData>> {
  return guard(async () => {
    await ensurePublished()
    return shareState()
  })
}

/**
 * Ensure published, mint (or reuse — the server is idempotent per scope) the
 * link for `scope`, copy it to the clipboard main-side, and return the URL.
 */
export async function shareCopyLink(scope: ShareScope): Promise<ShareResult<{ url: string }>> {
  return guard(async () => {
    if (!isScope(scope)) throw new Error(`invalid scope: ${String(scope)}`)
    const { serverUrl } = getCloudShareConfig()
    await ensurePublished()
    const binding = getSyncBinding()
    if (!binding) throw new Error('workspace is not published')
    const session = getStoredSession(serverUrl)
    if (!session) throw new Error('missing device session')

    const res = await fetch(`${serverUrl}/docs/${binding.docId}/links`, {
      method: 'POST',
      headers: ownerHeaders(session.cookie),
      body: JSON.stringify({ scope }),
    })
    if (!res.ok) throw new Error(`copy link failed (${res.status})`)
    const link = (await res.json()) as ServerLink
    const url = buildShareLink({ base: serverUrl, docId: binding.docId, token: link.token })
    clipboard.writeText(url)
    return { url }
  })
}

/**
 * Join someone else's canvas from a share link: redeem the grant for a
 * connection token, adopt their docId as this workspace's binding, attach.
 *
 * Refuses when the local workspace already holds entities. The runtime owns one
 * Y.Doc for the whole workspace, so attaching to a remote doc merges both
 * sides' entities in both directions — joining from a populated workspace would
 * silently union two people's canvases with no way back. Per-canvas doc
 * granularity (ADR 0018) is what lifts this restriction; until then an empty
 * workspace (a second profile via `--user-data-dir`) is the supported path.
 */
export async function shareJoin(link: string): Promise<ShareResult<ShareStateData>> {
  return guard(async () => {
    if (getSyncBinding()) throw new Error('this workspace is already published — join from an empty one')
    if (!isWorkspaceEmpty()) {
      throw new Error('joining would merge both canvases — join from an empty workspace')
    }

    const parsed = parseShareLink(link.trim())
    const connection = await redeemLink(parsed.base, parsed.token)
    publishBinding({ docId: connection.docId, url: parsed.base })
    connectSyncTransport(connection.token)
    return shareState()
  })
}

/** The active links for the published workspace. */
export async function shareListLinks(): Promise<ShareResult<ShareLinkInfo[]>> {
  return guard(async () => {
    const { serverUrl } = getCloudShareConfig()
    const binding = getSyncBinding()
    if (!binding) throw new Error('workspace is not published')
    const session = await ensureDeviceSession(serverUrl)
    return fetchLinks(serverUrl, session.cookie, binding.docId)
  })
}

/** Rotate a link's token in place — old link (and derived agent tokens) die. */
export async function shareResetLink(grantId: string): Promise<ShareResult<ShareLinkInfo>> {
  return guard(async () => {
    const { serverUrl } = getCloudShareConfig()
    const binding = getSyncBinding()
    if (!binding) throw new Error('workspace is not published')
    const session = await ensureDeviceSession(serverUrl)

    const res = await fetch(`${serverUrl}/docs/${binding.docId}/links/${grantId}/reset`, {
      method: 'POST',
      headers: ownerHeaders(session.cookie),
    })
    if (!res.ok) throw new Error(`reset link failed (${res.status})`)
    const link = (await res.json()) as ServerLink
    return toLinkInfo(serverUrl, binding.docId, link)
  })
}

/** Revoke a link — deletes the grant row. */
export async function shareRevokeLink(grantId: string): Promise<ShareResult<{ revoked: string }>> {
  return guard(async () => {
    const { serverUrl } = getCloudShareConfig()
    const binding = getSyncBinding()
    if (!binding) throw new Error('workspace is not published')
    const session = await ensureDeviceSession(serverUrl)

    const res = await fetch(`${serverUrl}/docs/${binding.docId}/links/${grantId}`, {
      method: 'DELETE',
      headers: ownerHeaders(session.cookie),
    })
    if (!res.ok) throw new Error(`revoke link failed (${res.status})`)
    const body = (await res.json()) as { revoked?: string }
    return { revoked: body.revoked ?? grantId }
  })
}
