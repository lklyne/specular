/**
 * Cloud-share types shared between the main-process share actions and the
 * toolbar renderer (ADR 0018 §4b). Pure types only — no runtime, no imports —
 * so `src/shared` stays side-effect-free and the renderer can consume them
 * without reaching into `src/main`.
 */

/** Capability-link roles (ADR 0018 §4, tier 2). Copy link defaults to comment. */
export type ShareScope = 'view' | 'comment' | 'edit'

/** One active capability link, resolved to a full, pasteable share URL. */
export interface ShareLinkInfo {
  grantId: string
  scope: ShareScope
  /** `<serverUrl>/c/<docId>#t=<token>` — the thing that goes on the clipboard. */
  url: string
}

/** Mirrors `SyncStatus` in `workspace-sync-state`; duplicated to keep shared pure. */
export type ShareStatus = 'off' | 'connecting' | 'connected' | 'error'

/** Everything the share popover renders from. */
export interface ShareStateData {
  /** The dev flag — the whole surface is hidden when false. */
  enabled: boolean
  serverUrl: string
  /** Present once the canvas has been published (has a cloud doc). */
  binding: { docId: string; url: string } | null
  status: ShareStatus
  /** Active links; omitted when unpublished or the server was unreachable. */
  links?: ShareLinkInfo[]
}

/**
 * Result envelope for every share action — errors travel as data, never as a
 * throw across the IPC boundary (a rejected `invoke` would surface as an opaque
 * renderer exception).
 */
export type ShareResult<T> = { ok: true; value: T } | { ok: false; error: string }
