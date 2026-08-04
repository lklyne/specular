/**
 * Entity-renderer registry (internal, v1).
 *
 * First-party plugins register a "claim" against persisted file entities. The
 * file-entity dispatch site asks the registry which renderer wins; the result
 * is either an inline tag (rendered renderer-side via <RendererSwitch/>) or
 * a wcv-page contract (the host creates a Page-like WebContentsView triple
 * pointed at the URL the plugin resolves).
 *
 * No public exports. The registry is mutated only by built-in plugins via
 * registerBuiltInPlugins() in src/main/plugins/index.ts.
 */

import type { PersistedFileEntity } from '../../shared/types'

export type EntityRendererKind = 'inline' | 'wcv-page'

export type EntityRendererTag = 'markdown' | 'image' | 'video' | 'component' | 'html'

interface BaseRendererClaim {
  /** Stable id used for telemetry, debugging, and unregister. */
  id: string
  /**
   * Renderer-side dispatch key, broadcast as part of the scene data so
   * canvas-bg/entity-renderers/RendererSwitch can pick a React component
   * without importing from src/main/. Required regardless of kind:
   * inline plugins use it to pick their inline component, wcv-page
   * plugins use it to pick a placeholder shown while the WCV materializes.
   */
  rendererTag: EntityRendererTag
  /** Pure predicate: does this plugin claim the entity? */
  claims: (entity: PersistedFileEntity) => boolean
  /**
   * Higher priority claims are tested first. Defaults to 0. Use a higher
   * value when the file pattern is more specific than another plugin's.
   * Same-priority claims fall back to registration order for tie-breaking.
   */
  priority?: number
  /** Whether the renderer has a meaningful inline-edit affordance. Drives
   *  the dblclick and click-on-solo-selected → edit paths. Renderers that
   *  ignore `canEdit` declare `false` so those gestures stay a clean no-op. */
  editable: boolean
  /** Whether the renderer hosts live content (an iframe) that gets the
   *  page-like select-first / interact-second treatment. Defaults to false. */
  interactive?: boolean
}

export interface InlineRendererClaim extends BaseRendererClaim {
  kind: 'inline'
  /** Whether the renderer supports a fullscreen 'fill' focus session — the
   *  note-style blow-up entered from `focusSelection`. Defaults to false. */
  fillFocus?: boolean
}

export interface WcvPageRendererClaim extends BaseRendererClaim {
  kind: 'wcv-page'
  /**
   * Produce the URL the page WebContents loads. Async because resolving
   * may require a dev-server lookup. Returning null tells the host to
   * render a placeholder.
   */
  resolveUrl: (entity: PersistedFileEntity) => Promise<string | null> | string | null
}

export type EntityRendererClaim = InlineRendererClaim | WcvPageRendererClaim

const claims: EntityRendererClaim[] = []

function priorityOf(claim: EntityRendererClaim): number {
  return claim.priority ?? 0
}

export function registerEntityRenderer(claim: EntityRendererClaim): void {
  if (claims.some((c) => c.id === claim.id)) {
    throw new Error(`entity renderer already registered: ${claim.id}`)
  }
  // Insertion sort: walk to the first existing claim with strictly lower
  // priority and insert there. Equal priorities keep registration order.
  const newPriority = priorityOf(claim)
  let index = claims.length
  for (let i = 0; i < claims.length; i++) {
    if (priorityOf(claims[i]) < newPriority) {
      index = i
      break
    }
  }
  claims.splice(index, 0, claim)
}

export function unregisterEntityRenderer(id: string): boolean {
  const index = claims.findIndex((c) => c.id === id)
  if (index < 0) return false
  claims.splice(index, 1)
  return true
}

/**
 * First registered claim that matches wins. Order is stable across the
 * process lifetime; built-in plugins register in a known order at boot.
 *
 * A throwing claims() predicate is logged and treated as "did not claim"
 * so one buggy plugin can't blank out every file behind it.
 */
export function pickRenderer(entity: PersistedFileEntity): EntityRendererClaim | null {
  for (const claim of claims) {
    try {
      if (claim.claims(entity)) return claim
    } catch (err) {
      console.error(
        `[entity-renderer] claim "${claim.id}" threw in claims(); skipping:`,
        err,
      )
    }
  }
  return null
}

/** Convenience: tag broadcast to the renderer; null when no plugin claims. */
export function getRendererTagFor(entity: PersistedFileEntity): EntityRendererTag | null {
  return pickRenderer(entity)?.rendererTag ?? null
}

/** Whether the claiming renderer opts into fullscreen 'fill' focus sessions. */
export function rendererClaimsFillFocus(claim: EntityRendererClaim | null): boolean {
  return claim?.kind === 'inline' && claim.fillFocus === true
}

/** Snapshot for debugging; not part of any IPC contract. */
export function listRegisteredRenderers(): readonly EntityRendererClaim[] {
  return claims
}

/** Test-only: drop all registrations. */
export function __resetRegistryForTests(): void {
  claims.length = 0
}
