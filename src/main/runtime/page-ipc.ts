import { ipcChannels } from '../../shared/ipc-contract'
import { randomUUID } from 'crypto'
import { findPageById, selectedPage } from './runtime-context'
import { safeSend } from './safe-send'

const pendingPageRequests = new Map<
  string,
  {
    pageId: string
    channel: string
    resolve: (value: unknown) => void
    reject: (reason: unknown) => void
    timer: NodeJS.Timeout
  }
>()

const pendingDetailRequests = new Map<string, { pageId: string; nodeId: string }>()

export function requestNodeDetail(pageId: string, nodeId: string): void {
  const page = findPageById(pageId)
  if (!page || page.pageView.webContents.isDestroyed()) return
  const requestId = randomUUID()
  pendingDetailRequests.set(requestId, { pageId, nodeId })
  safeSend(page.pageView.webContents, ipcChannels.resolveNodeDetail, { nodeId, requestId })
}

export function takePendingDetailRequest(
  requestId: string,
): { pageId: string; nodeId: string } | undefined {
  const request = pendingDetailRequests.get(requestId)
  if (!request) return undefined
  pendingDetailRequests.delete(requestId)
  return request
}

export function sendPageIpc(
  pageId: string | undefined,
  channel: string,
  payload: Record<string, unknown>,
): Promise<unknown> {
  const page = pageId ? findPageById(pageId) : selectedPage()
  if (!page || page.pageView.webContents.isDestroyed()) {
    return Promise.reject(
      new Error(pageId ? `Page not found: ${pageId}` : 'No page selected'),
    )
  }
  const resolvedPageId = page.id
  const requestId = randomUUID()
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pendingPageRequests.delete(requestId)
      reject(new Error(`Timed out waiting for ${channel}-response`))
    }, 5_000)
    pendingPageRequests.set(requestId, {
      pageId: resolvedPageId,
      channel,
      resolve,
      reject,
      timer,
    })
    safeSend(page.pageView.webContents, channel, { ...payload, requestId })
  })
}

// Drop any pending requests tied to a page that's being destroyed. Without
// this, the 5s timer in sendPageIpc keeps a closure over the doomed
// WebContents alive; when it fires later, V8 GC may sweep the already-freed
// native wrappable and segfault the main process.
export function clearPendingRequestsForPage(pageId: string): void {
  for (const [requestId, pending] of pendingPageRequests) {
    if (pending.pageId !== pageId) continue
    clearTimeout(pending.timer)
    pendingPageRequests.delete(requestId)
    pending.reject(new Error(`Page ${pageId} destroyed before ${pending.channel}-response`))
  }
  for (const [requestId, detail] of pendingDetailRequests) {
    if (detail.pageId === pageId) pendingDetailRequests.delete(requestId)
  }
}

export function handlePageIpcResponse(payload: {
  requestId: string
  data: unknown
}): void {
  const pending = pendingPageRequests.get(payload.requestId)
  if (!pending) return
  pendingPageRequests.delete(payload.requestId)
  clearTimeout(pending.timer)
  pending.resolve(payload.data)
}
