import { ipcChannels } from '../../shared/ipc-contract'
import { selectedPage, findPageById } from './runtime-context'
import { sendPageIpc } from './page-ipc'

export async function takePageSnapshot(pageId?: string, maxDepth?: number): Promise<unknown> {
  return sendPageIpc(pageId, ipcChannels.takeDomSnapshot, { maxDepth: maxDepth ?? 10 })
}

export async function takePageAgentSnapshot(pageId?: string, maxDepth?: number): Promise<unknown> {
  return sendPageIpc(pageId, ipcChannels.takeDomSnapshot, {
    maxDepth: maxDepth ?? 10,
    structured: true,
  })
}

export async function takePageScreenshot(pageId?: string): Promise<string> {
  const page = pageId ? findPageById(pageId) : selectedPage()
  if (!page || page.pageView.webContents.isDestroyed()) {
    throw new Error(pageId ? `Page not found: ${pageId}` : 'No page selected')
  }
  const image = await page.pageView.webContents.capturePage()
  return image.toPNG().toString('base64')
}

export async function queryPageElements(pageId?: string, selector?: string, maxResults?: number): Promise<unknown> {
  if (!selector) throw new Error('selector is required')
  return sendPageIpc(pageId, ipcChannels.queryDomElements, { selector, maxResults: maxResults ?? 20 })
}

export async function queryElementsInRect(
  pageId: string,
  rect: { x: number; y: number; width: number; height: number },
  maxResults?: number,
): Promise<unknown[]> {
  const data = await sendPageIpc(pageId, ipcChannels.queryElementsInRect, {
    rect,
    maxResults: maxResults ?? 15,
  })
  return Array.isArray(data) ? data : []
}

/**
 * Ask a page to capture the reference element at a *document* point (ADR 0030
 * element attachment). Returns the element's selector plus its document
 * position, or `null` if the page has no body / is gone. Fire-and-forget
 * enrichment — callers ignore rejection (timeout, destroyed page).
 */
export async function captureElementAtPageDocumentPoint(
  pageId: string,
  docX: number,
  docY: number,
): Promise<{ selector: string; docX: number; docY: number } | null> {
  const data = await sendPageIpc(pageId, ipcChannels.captureElementAtPoint, { docX, docY })
  if (!data || typeof data !== 'object') return null
  return data as { selector: string; docX: number; docY: number }
}

/**
 * Resolve the element under a page-local (x, y) coordinate. Used by the
 * unified comment tool's click-vs-element-anchor flow (ADR 0006). Returns
 * `null` if the page has no element at that point or the page is gone.
 */
export async function queryElementAtPoint(
  pageId: string,
  x: number,
  y: number,
): Promise<Record<string, unknown> | null> {
  try {
    const data = await sendPageIpc(pageId, ipcChannels.queryElementAtPoint, { x, y })
    if (!data || typeof data !== 'object') return null
    return data as Record<string, unknown>
  } catch {
    return null
  }
}
