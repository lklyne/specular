/**
 * Decouples a page's window-open handler (page-factory) from the duplicate
 * flow (workspace-pages). When a link would open a new tab, page-factory calls
 * `openLinkInNewFrame`; the real handler is registered once at startup. Routing
 * through this setter keeps page-factory free of any import edge back into
 * workspace-pages, which would form an initialization cycle.
 */
export type OpenLinkInNewFrame = (input: {
  sourcePageId: string
  url: string
}) => void

let handler: OpenLinkInNewFrame | null = null

export function setOpenLinkInNewFrameHandler(fn: OpenLinkInNewFrame): void {
  handler = fn
}

export function openLinkInNewFrame(input: {
  sourcePageId: string
  url: string
}): void {
  handler?.(input)
}
