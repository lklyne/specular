/**
 * Headless peer session: joins a canvas Durable Object with its own `Y.Doc`
 * and writes/repoints HTML file entities into the shared doc — the write half
 * of the HTML prototyping loop (ADR 0018 §5). Pure node; drives the same
 * `y-partyserver` `YProvider` the desktop transport uses.
 *
 * The doc-entry it writes MUST match the runtime's persisted file-entity shape
 * (`persistFileEntity` in `file-entity-state.ts`): the entity lands in the
 * `entities` Y.Map keyed by id, with its id appended to the `entityOrder`
 * array, so a connected desktop reconstructs the same runtime entity. Content
 * addressing is the reload signal — repointing `file` to a new `asset://<hash>`
 * is the whole update; no version counter.
 */

import { randomUUID } from 'node:crypto'
import WebSocket from 'ws'
import YProvider from 'y-partyserver/provider'
import * as Y from 'yjs'
import { DOC_MAP_ENTITIES, DOC_ARRAY_ENTITY_ORDER } from '../runtime/workspace-doc'
import { parseShareLink, redeemLink, uploadAsset, type ConnectionToken } from './share-link'

const CANVAS_DOC_PARTY = 'canvas-doc'
const DEFAULT_ENTITY_SIZE = 400

export interface HtmlEntityPlacement {
  canvasX?: number
  canvasY?: number
  width?: number
  height?: number
  /** Repoint an existing entity when set; otherwise a fresh id is minted. */
  id?: string
}

export interface HtmlEntityResult {
  docId: string
  entityId: string
  assetId: string
}

/**
 * A live headless connection to one canvas doc. `join` redeems a link and opens
 * the provider; callers upload HTML and `close` when done. Not concurrency-safe
 * for its own doc writes — one agent, one linear flow.
 */
export class SyncClientSession {
  readonly base: string
  readonly docId: string
  readonly doc: Y.Doc
  readonly provider: YProvider
  private readonly connToken: string

  private constructor(args: {
    base: string
    docId: string
    connToken: string
    doc: Y.Doc
    provider: YProvider
  }) {
    this.base = args.base
    this.docId = args.docId
    this.connToken = args.connToken
    this.doc = args.doc
    this.provider = args.provider
  }

  /** Parse a share link, redeem it, and open the DO connection. */
  static async join(link: string): Promise<SyncClientSession> {
    const parsed = parseShareLink(link)
    const conn: ConnectionToken = await redeemLink(parsed.base, parsed.token)
    const doc = new Y.Doc()
    const provider = new YProvider(hostOf(parsed.base), conn.docId, doc, {
      party: CANVAS_DOC_PARTY,
      params: { token: conn.token },
      WebSocketPolyfill: WebSocket as unknown as typeof globalThis.WebSocket,
      disableBc: true,
    })
    const session = new SyncClientSession({
      base: parsed.base,
      docId: conn.docId,
      connToken: conn.token,
      doc,
      provider,
    })
    await session.waitSynced()
    return session
  }

  /** Resolve once the provider has completed the initial state exchange. */
  waitSynced(timeoutMs = 10_000): Promise<void> {
    if (this.provider.synced) return Promise.resolve()
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.provider.off('sync', onSync)
        reject(new Error('sync-client: timed out waiting for initial sync'))
      }, timeoutMs)
      const onSync = (isSynced: boolean) => {
        if (!isSynced) return
        clearTimeout(timer)
        this.provider.off('sync', onSync)
        resolve()
      }
      this.provider.on('sync', onSync)
    })
  }

  /**
   * Upload `html` and write (or repoint, when `place.id` is set) an HTML file
   * entity pointing at the content hash. Awaits the outgoing update flushing to
   * the DO so the caller may exit knowing the change is in flight.
   */
  async writeHtmlEntity(html: string | Uint8Array, place: HtmlEntityPlacement = {}): Promise<HtmlEntityResult> {
    const bytes = typeof html === 'string' ? new TextEncoder().encode(html) : html
    const assetId = await uploadAsset(this.base, this.docId, this.connToken, bytes, 'text/html')
    const file = `asset://${assetId}.html`
    const entityId = place.id ?? `file_${randomUUID()}`

    const entities = this.doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>
    const order = this.doc.getArray<string>(DOC_ARRAY_ENTITY_ORDER)
    this.doc.transact(() => {
      const existing = entities.get(entityId)
      if (existing) {
        existing.set('file', file)
      } else {
        entities.set(entityId, buildFileEntityYMap({ id: entityId, file, place }))
        if (!order.toArray().includes(entityId)) order.push([entityId])
      }
    }, 'agent')

    await this.flush()
    return { docId: this.docId, entityId, assetId }
  }

  /** Entity-kind → count summary of the doc, for a cheap liveness read. */
  summary(): { entityCounts: Record<string, number> } {
    const entities = this.doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>
    const counts: Record<string, number> = {}
    for (const [, ym] of entities.entries()) {
      const kind = (ym.get('kind') as string) ?? 'unknown'
      counts[kind] = (counts[kind] ?? 0) + 1
    }
    return { entityCounts: counts }
  }

  /** Wait for the WebSocket send buffer to drain (best-effort flush). */
  async flush(timeoutMs = 5_000): Promise<void> {
    const start = Date.now()
    while (this.provider.ws && this.provider.ws.bufferedAmount > 0) {
      if (Date.now() - start > timeoutMs) break
      await delay(10)
    }
  }

  close(): void {
    this.provider.destroy()
  }
}

/** The provider host is scheme-less; it derives ws/wss from the host. */
function hostOf(base: string): string {
  return new URL(base).host
}

/**
 * Minimal persisted file-entity Y.Map. Mirrors `persistFileEntity`'s field set
 * (`file-entity-state.ts`) — undefined fields are omitted, exactly as the doc's
 * `objectToYMap` does — so a connected desktop restores it as a real entity.
 */
function buildFileEntityYMap(args: { id: string; file: string; place: HtmlEntityPlacement }): Y.Map<unknown> {
  const m = new Y.Map<unknown>()
  m.set('kind', 'file')
  m.set('id', args.id)
  m.set('file', args.file)
  m.set('canvasX', args.place.canvasX ?? 0)
  m.set('canvasY', args.place.canvasY ?? 0)
  m.set('width', args.place.width ?? DEFAULT_ENTITY_SIZE)
  m.set('height', args.place.height ?? DEFAULT_ENTITY_SIZE)
  return m
}

const delay = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms))
