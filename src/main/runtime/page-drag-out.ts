/**
 * Drag-out from a page onto the canvas (ADR 0038, "drag-out"). An offscreen
 * page can't hand a native drag off to the OS — `StartDragging` is a no-op
 * for an offscreen view, while the page keeps receiving forwarded mouse
 * moves with the button held — so the page's own `dragstart` (captured by
 * the page-content preload) arms a pending payload here instead. If aboveView
 * later reports the forwarded pointer released outside that page's content,
 * the armed payload is consumed and turned into a canvas entity at the
 * release point. Released inside the page, the drag is simply lost — the
 * documented limitation (ADR 0038 open question 2).
 */

import { net } from 'electron'
import type { PageDragPayload } from '../../shared/types'
import { ipcChannels } from '../../shared/ipc-contract'
import { decodePercentEncoded, dragMayLoad } from './page-drag-policy'
import { findPageById } from './runtime-context'
import { aboveView } from './view-refs'
import { safeSend } from './safe-send'
import { saveImageBuffer } from './image-assets'
import { imageSizeFromBuffer } from './image-sizing'
import { createFileEntity } from './document-commands'
import { createTextEntity } from './text-entity-state'
import { getStickyDefaultColor, getStickyDefaultFont, getStickyDefaultSize } from './tool-defaults'
import { createPageAtPosition } from '../workspace-pages'

const PENDING_DRAG_TTL_MS = 10_000
const MAX_DRAG_IMAGE_BYTES = 25 * 1024 * 1024
const FETCH_TIMEOUT_MS = 10_000

interface PendingPageDrag {
  pageId: string
  payload: PageDragPayload
  startedAt: number
}

/** One drag at a time — a second `dragstart` before the first resolves just
 *  replaces it, matching how a single native drag session works. */
let pendingDrag: PendingPageDrag | null = null

/** Called from the `page-drag-start` IPC handler once a page's dragstart is
 *  mapped back to its page id. */
export function armPageDrag(pageId: string, payload: PageDragPayload): void {
  pendingDrag = { pageId, payload, startedAt: Date.now() }
  if (!aboveView || aboveView.webContents.isDestroyed()) return
  safeSend(aboveView.webContents, ipcChannels.pageDragArmed, { pageId, payload })
}

function takePendingDrag(pageId: string): PageDragPayload | null {
  const pending = pendingDrag
  if (!pending) return null
  pendingDrag = null
  if (pending.pageId !== pageId) return null
  if (Date.now() - pending.startedAt > PENDING_DRAG_TTL_MS) return null
  return pending.payload
}

/** Called from the `canvas-drop-page-drag` IPC handler when aboveView
 *  decides a forwarded pointer-up landed outside the source page's content. */
export async function dropPageDragOnCanvas(input: {
  pageId: string
  canvasX: number
  canvasY: number
}): Promise<{ createdId: string } | null> {
  const payload = takePendingDrag(input.pageId)
  if (!payload) return null
  const sourcePageUrl = findPageById(input.pageId)?.url

  if (payload.kind === 'text') {
    const entity = createTextEntity({
      canvasX: input.canvasX,
      canvasY: input.canvasY,
      text: payload.text,
      textStyle: 'sticky',
      color: getStickyDefaultColor(),
      textSize: getStickyDefaultSize(),
      textFont: getStickyDefaultFont(),
    })
    return { createdId: entity.id }
  }

  if (payload.kind === 'link') {
    if (!dragMayLoad(payload.url, sourcePageUrl)) return null
    const sourcePage = findPageById(input.pageId)
    const { pageId } = createPageAtPosition({
      presetIndex: sourcePage?.presetIndex ?? 0,
      canvasX: input.canvasX,
      canvasY: input.canvasY,
      mode: 'paste_url',
      focus: false,
      url: payload.url,
    })
    return { createdId: pageId }
  }

  const buffer = await fetchDragImageBuffer(payload.src, sourcePageUrl)
  if (!buffer) return null
  const file = saveImageBuffer(buffer, extensionForImage(payload.src))
  const { width, height } = imageSizeFromBuffer(buffer)
  const entity = createFileEntity({ canvasX: input.canvasX, canvasY: input.canvasY, file, width, height })
  return { createdId: entity.id }
}

/** `net.fetch` doesn't support `data:` (Electron limitation), so that scheme
 *  is decoded directly rather than round-tripped through the network stack. */
async function fetchDragImageBuffer(
  src: string,
  sourcePageUrl: string | undefined,
): Promise<Buffer | null> {
  if (src.startsWith('data:')) return decodeDataUrlImage(src)
  if (!dragMayLoad(src, sourcePageUrl)) return null

  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS)
  try {
    const response = await net.fetch(src, { signal: controller.signal })
    if (!response.ok || !response.body) return null
    const declaredLength = Number(response.headers.get('content-length') ?? '0')
    if (declaredLength > MAX_DRAG_IMAGE_BYTES) return null
    const buffer = Buffer.from(await response.arrayBuffer())
    if (buffer.byteLength > MAX_DRAG_IMAGE_BYTES) return null
    return buffer
  } catch {
    return null
  } finally {
    clearTimeout(timeout)
  }
}

function decodeDataUrlImage(dataUrl: string): Buffer | null {
  const match = /^data:([^;,]*)(;base64)?,(.*)$/s.exec(dataUrl)
  if (!match) return null
  const [, , isBase64, data] = match
  const buffer = isBase64 ? Buffer.from(data, 'base64') : decodePercentEncoded(data)
  return buffer.byteLength > MAX_DRAG_IMAGE_BYTES ? null : buffer
}

const IMAGE_MIME_EXTENSIONS: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/gif': 'gif',
  'image/webp': 'webp',
  'image/svg+xml': 'svg',
  'image/bmp': 'bmp',
  'image/avif': 'avif',
}

function extensionForImage(src: string): string {
  if (src.startsWith('data:')) {
    const mime = /^data:([^;,]+)/.exec(src)?.[1]
    return (mime && IMAGE_MIME_EXTENSIONS[mime]) || 'png'
  }
  try {
    const ext = new URL(src).pathname.split('.').pop()?.toLowerCase()
    return ext && /^[a-z0-9]{2,5}$/.test(ext) ? ext : 'png'
  } catch {
    return 'png'
  }
}
