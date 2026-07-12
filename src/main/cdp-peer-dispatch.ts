import type { Page } from './runtime/runtime-entities'
import {
  type ClickScaleSnapshot,
  createClickScaleSnapshot,
  compensateMousePointForDispatch,
} from './cdp-input-compensation'

// Interaction-sync peer dispatcher (ADR 0030, D8). Sibling of the agent CDP
// path in app-control-server — NOT routed through the CDP proxy. Replays a
// confidently-resolved hover/click onto a sync peer as trusted input via the
// peer's own `webContents.debugger`, at the peer-resolved point. Per-peer
// failures (attach denied, view torn down mid-fan-out) are caught and logged,
// never thrown across the fan-out.

async function dispatchMouse(
  page: Page,
  cdpType: 'mouseMoved' | 'mousePressed' | 'mouseReleased',
  point: { x: number; y: number },
  snapshot: ClickScaleSnapshot,
): Promise<void> {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return
  const compensated = compensateMousePointForDispatch(snapshot, cdpType, point)
  const isMove = cdpType === 'mouseMoved'
  if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')
  await wc.debugger.sendCommand('Input.dispatchMouseEvent', {
    type: cdpType,
    x: compensated.x,
    y: compensated.y,
    button: isMove ? 'none' : 'left',
    clickCount: isMove ? 0 : 1,
  })
}

/** Replay a confident hover as a trusted `mouseMoved` so the peer gets real
 *  `:hover` states at its own resolved element point. */
export async function dispatchPeerHover(
  page: Page,
  point: { x: number; y: number },
): Promise<void> {
  try {
    await dispatchMouse(page, 'mouseMoved', point, createClickScaleSnapshot())
  } catch (error) {
    console.warn('[interaction-sync] peer hover dispatch failed', {
      pageId: page.id,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}

/** Replay a confident click as a trusted press+release pair at the peer's own
 *  resolved point. The pair shares one snapshotted emulation scale. */
export async function dispatchPeerClick(
  page: Page,
  point: { x: number; y: number },
): Promise<void> {
  const snapshot = createClickScaleSnapshot()
  try {
    await dispatchMouse(page, 'mousePressed', point, snapshot)
    await dispatchMouse(page, 'mouseReleased', point, snapshot)
  } catch (error) {
    console.warn('[interaction-sync] peer click dispatch failed', {
      pageId: page.id,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
