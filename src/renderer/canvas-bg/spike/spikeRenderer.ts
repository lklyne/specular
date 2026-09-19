import type { PageFrameMeta } from '../../../shared/page-frames'

/** One page to draw this pass. All rect values are in DEVICE pixels of the spike canvas. */
export interface SpikePageDraw {
  pageId: string
  frame: VideoFrame
  meta: PageFrameMeta
  x: number
  y: number
  width: number
  height: number
  /** Corner radius in device px. Renderers may ignore it. */
  radius: number
}

export interface SpikePageRenderer {
  /** A new frame replaced the page's held frame. The previous frame is already closed. Called before the paced draw. */
  frameArrived(pageId: string, frame: VideoFrame, meta: PageFrameMeta): void
  /** The page left the scene; its frame is already closed. Free per-page GPU resources. */
  pageRemoved(pageId: string): void
  /** Draw every page, in order (last is topmost), onto the canvas. `canvasWidth/Height` are device px; the canvas backing store is already sized to them. Must clear to transparent first. */
  draw(pages: readonly SpikePageDraw[], canvasWidth: number, canvasHeight: number): void
  dispose(): void
}

export type SpikePageRendererFactory = (canvas: HTMLCanvasElement) => Promise<SpikePageRenderer>
