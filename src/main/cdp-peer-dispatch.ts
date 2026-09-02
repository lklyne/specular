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
  // A move with the left button held is a drag, not a hover: Chromium reads
  // the held button from the `buttons` bitmask, and a move that reports `none`
  // ends the gesture as far as the page is concerned.
  buttonHeld = false,
): Promise<void> {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return
  const compensated = compensateMousePointForDispatch(snapshot, cdpType, point)
  const isMove = cdpType === 'mouseMoved'
  const leftDown = cdpType === 'mousePressed' || (isMove && buttonHeld)
  if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')
  await wc.debugger.sendCommand('Input.dispatchMouseEvent', {
    type: cdpType,
    x: compensated.x,
    y: compensated.y,
    button: isMove && !buttonHeld ? 'none' : 'left',
    buttons: leftDown ? 1 : 0,
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
  let pressed = false
  try {
    await dispatchMouse(page, 'mousePressed', point, snapshot)
    pressed = true
    await dispatchMouse(page, 'mouseReleased', point, snapshot)
  } catch (error) {
    console.warn('[interaction-sync] peer click dispatch failed', {
      pageId: page.id,
      error: error instanceof Error ? error.message : String(error),
    })
    if (pressed) {
      // The press landed but the release rejected — the peer now holds a
      // phantom left button, so the next mouseMoved would read as a drag.
      // Best-effort compensating release (a fresh snapshot; position barely
      // matters, only the button-up does).
      try {
        await dispatchMouse(page, 'mouseReleased', point, createClickScaleSnapshot())
      } catch {
        // Nothing more we can do; the debugger is likely gone.
      }
    }
  }
}

/**
 * A peer's in-flight surface drag. Owns the emulation-scale snapshot for the
 * whole gesture (so a mid-drag zoom can't split it across two scales) and the
 * last point dispatched, which is where the release lands.
 */
export interface PeerDragSession {
  snapshot: ClickScaleSnapshot
  point: { x: number; y: number }
}

/** Press the left button at the peer's own resolved point, opening a drag.
 *  Returns null if the press failed — the caller then has no session and will
 *  neither move nor release. */
export async function dispatchPeerDragStart(
  page: Page,
  point: { x: number; y: number },
): Promise<PeerDragSession | null> {
  const session: PeerDragSession = { snapshot: createClickScaleSnapshot(), point }
  try {
    await dispatchMouse(page, 'mousePressed', point, session.snapshot)
    return session
  } catch (error) {
    console.warn('[interaction-sync] peer drag start failed', {
      pageId: page.id,
      error: error instanceof Error ? error.message : String(error),
    })
    return null
  }
}

/** Continue a drag: a move with the button still held, at the peer's own
 *  proportional point inside the latched surface rect. */
export async function dispatchPeerDragMove(
  page: Page,
  session: PeerDragSession,
  point: { x: number; y: number },
): Promise<void> {
  session.point = point
  try {
    await dispatchMouse(page, 'mouseMoved', point, session.snapshot, true)
  } catch (error) {
    console.warn('[interaction-sync] peer drag move failed', {
      pageId: page.id,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}

/** Release the drag at its last point. Always attempted for a session that
 *  opened, including on teardown paths (unsync, navigation, source exit) — a
 *  peer left holding the button reads every later move as a continuing drag. */
export async function dispatchPeerDragEnd(page: Page, session: PeerDragSession): Promise<void> {
  try {
    await dispatchMouse(page, 'mouseReleased', session.point, session.snapshot)
  } catch (error) {
    console.warn('[interaction-sync] peer drag end failed', {
      pageId: page.id,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
