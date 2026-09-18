// fallow-ignore-file circular-dependencies
// Suppressed: see #141. viewport-control → layout-engine import component-page-factory back
/**
 * Component page factory.
 *
 * For each file entity that the registry tags as `component`, owns a
 * dedicated WebContentsView pointed at the dev-server URL the
 * component-render plugin resolves. The host treats these views the
 * same way it treats Page WCVs at layout time — see
 * `layoutComponentView` in layout-engine — but they have no chrome,
 * no devtools-host slot, and no navigation propagation. The dev URL
 * is the only thing they ever load.
 *
 * Lifecycle is driven by `syncComponentViews(entities)`, which the
 * layout pass calls before iterating per-component bounds. Creates
 * views for newly-tagged entities, destroys views whose entity is
 * gone or no longer claims `component`, and retries URL resolution
 * for any view still waiting on a dev server when the repo finishes
 * starting.
 */

import { WebContentsView } from 'electron'
import { win } from './view-refs'
import { findRepoForPath, onChange as onRepoChange } from './dev-server-manager'
import { pickRenderer } from '../plugins/registry'
import { wireRendererLogging } from '../crash-log'
import { CARD_BORDER_RADIUS } from './runtime-constants'
import { requestLayout } from './viewport-control'
import { persistFileEntity, type FileEntity } from './file-entity-state'
import { installScrollbarCss } from './page-scrollbar-css'

export interface ComponentView {
  entityId: string
  view: WebContentsView
  /** Last URL successfully passed to loadURL, or null while unresolved. */
  loadedUrl: string | null
  /** True while a resolveUrl() promise is in flight. Prevents redundant retries. */
  resolving: boolean
  /** Bounds key from layout-engine's last setBounds call. */
  lastBoundsKey?: string | null
  /** Emulation key from layout-engine's last enableDeviceEmulation call. */
  lastEmulationKey?: string | null
}

const componentViews = new Map<string, ComponentView>()

let repoChangeUnsubscribe: (() => void) | null = null

function ensureRepoSubscription(): void {
  if (repoChangeUnsubscribe) return
  repoChangeUnsubscribe = onRepoChange(() => {
    // A repo flipped state. Any view still waiting on a URL gets another shot.
    for (const cv of componentViews.values()) {
      if (cv.loadedUrl !== null || cv.resolving) continue
      void resolveAndLoad(cv)
    }
  })
}

function createView(entityId: string): ComponentView {
  if (!win) throw new Error('Window not initialized')
  const view = new WebContentsView({
    webPreferences: {
      focusOnNavigation: false,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })
  view.setBackgroundColor('#00000000')
  view.setBorderRadius(CARD_BORDER_RADIUS)
  view.webContents.loadURL('about:blank').catch(() => {})
  // Attachment is owned by the layout pass child-list reconcile.
  wireRendererLogging(view.webContents, `component:${entityId}`)
  installScrollbarCss(view.webContents)
  view.webContents.on('did-finish-load', () => {
    requestLayout()
  })
  return {
    entityId,
    view,
    loadedUrl: null,
    resolving: false,
  }
}

async function resolveAndLoad(cv: ComponentView): Promise<void> {
  const entity = currentEntities.get(cv.entityId)
  if (!entity) return
  const persisted = persistFileEntity(entity)
  const claim = pickRenderer(persisted)
  if (!claim || claim.kind !== 'wcv-page') return
  cv.resolving = true
  try {
    const url = await claim.resolveUrl(persisted)
    if (!url) return
    if (cv.view.webContents.isDestroyed()) return
    if (cv.loadedUrl === url) return
    cv.loadedUrl = url
    cv.view.webContents.loadURL(url).catch(() => {
      // Reset so the next repo-change tick can retry.
      cv.loadedUrl = null
    })
  } finally {
    cv.resolving = false
  }
}

/**
 * Snapshot of file entities the factory should know about. Updated by
 * syncComponentViews so resolveAndLoad can look up the current shape
 * without taking a dependency on file-entity-state's mutable array.
 */
const currentEntities = new Map<string, FileEntity>()

function destroyView(cv: ComponentView): void {
  // Detachment is owned by the layout pass child-list reconcile.
  if (!cv.view.webContents.isDestroyed()) {
    cv.view.webContents.close()
  }
  componentViews.delete(cv.entityId)
}

function shouldHaveComponentView(entity: FileEntity): boolean {
  const claim = pickRenderer(persistFileEntity(entity))
  if (claim?.kind !== 'wcv-page' || claim.rendererTag !== 'component') return false
  // Tie WCV existence to repo connectivity so disconnect tears down the live
  // view (and reconnect rebuilds it) without leaving stale dev-server URLs
  // loaded behind the placeholder.
  return findRepoForPath(entity.file) !== null
}

/**
 * Reconcile componentViews against the current file entities. Idempotent —
 * safe to call from layout-engine on every pass. Cheap when nothing changed
 * (Map lookups, no allocations).
 */
export function syncComponentViews(entities: readonly FileEntity[]): void {
  ensureRepoSubscription()

  // Refresh the snapshot up front so resolveAndLoad sees current metadata.
  currentEntities.clear()
  const desired = new Set<string>()
  for (const entity of entities) {
    currentEntities.set(entity.id, entity)
    if (shouldHaveComponentView(entity)) desired.add(entity.id)
  }

  // Drop views whose entity is gone or no longer wants one.
  for (const [id, cv] of componentViews) {
    if (!desired.has(id)) destroyView(cv)
  }

  // Spin up views for newly-component entities and kick off URL resolution.
  // The layout pass child-list reconcile (which calls syncComponentViews)
  // owns attach/detach — it runs every pass, so any view-set delta here is
  // picked up unconditionally without a dirty flag.
  for (const id of desired) {
    if (componentViews.has(id)) continue
    const cv = createView(id)
    componentViews.set(id, cv)
    void resolveAndLoad(cv)
  }

  // Component-render metadata can change in place (e.g. user moves a file
  // into a connected repo's tree). Re-resolve any view whose loaded URL
  // doesn't reflect the current metadata.
  for (const cv of componentViews.values()) {
    if (cv.resolving) continue
    if (cv.loadedUrl !== null) continue
    void resolveAndLoad(cv)
  }
}

export function getComponentView(entityId: string): ComponentView | null {
  return componentViews.get(entityId) ?? null
}

export function listComponentViews(): readonly ComponentView[] {
  return Array.from(componentViews.values())
}

export async function destroyAllComponentViews(): Promise<void> {
  for (const cv of Array.from(componentViews.values())) {
    destroyView(cv)
  }
  if (repoChangeUnsubscribe) {
    repoChangeUnsubscribe()
    repoChangeUnsubscribe = null
  }
  currentEntities.clear()
}

/** Test-only: expose internal state for assertions. */
export function __debugComponentFactoryState(): {
  views: number
  entities: number
} {
  return {
    views: componentViews.size,
    entities: currentEntities.size,
  }
}
