/**
 * Decouples a page's window-open handler (page-factory) from the duplicate
 * flow (workspace-pages). When a link would open a new tab, page-factory calls
 * `openLinkInNewFrame`; the real handler is registered once at startup. Routing
 * through this setter keeps page-factory free of any import edge back into
 * workspace-pages, which would form an initialization cycle.
 */
export type OpenLinkInput = { sourcePageId: string; url: string }
export type OpenLinkInNewFrame = (input: OpenLinkInput) => void

let handler: OpenLinkInNewFrame | null = null

export function setOpenLinkInNewFrameHandler(fn: OpenLinkInNewFrame): void {
  handler = fn
}

export function openLinkInNewFrame(input: OpenLinkInput): void {
  // No-op until the handler is registered at startup (index.ts); a missing
  // handler simply means no frame is spawned.
  handler?.(input)
}
