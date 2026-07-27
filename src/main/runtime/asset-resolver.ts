/**
 * Asset resolver (ADR 0018 §3, cloud-sync spike step 5).
 *
 * A file entity that references cloud-portable content stores a stable,
 * content-addressed asset id — never a location. `asset://<sha256hex>[.<ext>]`
 * is the canonical reference form; this module is the single seam that maps
 * that id to a location per environment: a local `assets/<hash>[.<ext>]` path
 * on desktop, or the sync server's asset endpoint once a canvas is published.
 * Same shape as the `page` ⇄ JSON Canvas `link` seam (ADR 0003) — one place
 * the two encodings meet, so every other module keeps working with whichever
 * shape it already understood (a filesystem path, or an http(s) URL).
 */

import { readdirSync } from 'fs'
import { join } from 'path'

const ASSET_SCHEME = 'asset://'
const HASH_PATTERN = /^[0-9a-f]{64}$/i

export interface AssetReference {
  hash: string
  ext?: string
}

export type ResolvedAsset = { kind: 'local'; path: string } | { kind: 'remote'; url: string }

export function isAssetReference(file: string): boolean {
  return file.startsWith(ASSET_SCHEME)
}

/**
 * Parse `asset://<hash>[.<ext>]` into its hash and optional extension. Returns
 * null for anything that isn't a well-formed content-addressed reference —
 * callers treat that the same as "not an asset reference at all".
 */
export function parseAssetReference(file: string): AssetReference | null {
  if (!isAssetReference(file)) return null
  const rest = file.slice(ASSET_SCHEME.length)
  const dot = rest.indexOf('.')
  const hash = dot === -1 ? rest : rest.slice(0, dot)
  const ext = dot === -1 ? undefined : rest.slice(dot + 1)
  if (!HASH_PATTERN.test(hash)) return null
  if (ext !== undefined && ext.length === 0) return null
  return { hash, ext }
}

export interface AssetResolveContext {
  /** Base URL of the sync server this workspace is bound to (workspace-sync's
   *  `SyncBinding.url`); null/undefined when the workspace isn't published. */
  syncBaseUrl?: string | null
  /** Workspace `assets/` directory to probe for a local copy; null skips the
   *  local probe (no electron runtime to resolve the workspace dir). */
  localAssetsDir: string | null
  /** Injected directory listing so the core stays pure and unit-testable
   *  without a real filesystem; defaults to `fs.readdirSync`. */
  listDir?: (dir: string) => string[]
}

/**
 * Resolve an `asset://` reference to a location for this process.
 *
 * Local wins when a matching file already sits in the workspace `assets/`
 * dir — no network round-trip for content this machine already has.
 * Otherwise falls back to the sync server's content-addressed asset route
 * (`GET <syncBaseUrl>/assets/<hash>`) when a binding exists. Returns null
 * when neither is available (no local copy, no sync binding) — the caller
 * decides how to degrade, since "unresolvable" isn't this module's call.
 */
export function resolveAssetReference(file: string, ctx: AssetResolveContext): ResolvedAsset | null {
  const ref = parseAssetReference(file)
  if (!ref) return null

  const local = findLocalAsset(ref, ctx)
  if (local) return { kind: 'local', path: local }

  if (ctx.syncBaseUrl) {
    return { kind: 'remote', url: `${ctx.syncBaseUrl.replace(/\/+$/, '')}/assets/${ref.hash}` }
  }

  return null
}

function findLocalAsset(ref: AssetReference, ctx: AssetResolveContext): string | null {
  if (ctx.localAssetsDir === null) return null
  const listDir = ctx.listDir ?? safeReaddir
  const names = listDir(ctx.localAssetsDir)
  if (ref.ext) {
    const exact = `${ref.hash}.${ref.ext}`
    if (names.includes(exact)) return join(ctx.localAssetsDir, exact)
  }
  const match = names.find((name) => name === ref.hash || name.startsWith(`${ref.hash}.`))
  return match ? join(ctx.localAssetsDir, match) : null
}

function safeReaddir(dir: string): string[] {
  try {
    return readdirSync(dir)
  } catch {
    return []
  }
}
