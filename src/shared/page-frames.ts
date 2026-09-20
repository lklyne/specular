/**
 * The wire contract for one painted page frame: main imports the GPU texture
 * a page's offscreen window produced and sends it to canvas-bg's main frame
 * with this metadata attached.
 */

export interface PageFrameMeta {
  pageId: string
  widgetType: 'frame' | 'popup'
  /** Texture size in device pixels. */
  width: number
  height: number
  /**
   * CSS viewport size the frame was painted for. It trails the page's size by
   * a few frames through a resize, so the canvas draws a frame at this size
   * rather than fitting it to the page's rect.
   */
  cssWidth: number
  cssHeight: number
  /**
   * The rate the page is painting at (its LOD tier, `page-frame-rate.ts`).
   * The canvas repaints whole for any one page's frame, so it paces those
   * repaints to the fastest rate arriving rather than to every arrival.
   */
  frameRate: number
  /**
   * How much input the page had been sent when this frame was painted. A
   * popup closes because of something the user did, so this is what tells a
   * page paint that follows a dismissal apart from one an animation produced
   * (`page-input-counter.ts`).
   */
  inputSeq: number
}

/** Posted by the canvas-bg preload to the page world, the bitmap transferred with it. */
export interface PageFrameMessage {
  source: 'page-frame'
  meta: PageFrameMeta
  bitmap: ImageBitmap
}

/** A focused element's rect in page CSS px — the point a popup widget hangs from. */
export interface PagePopupAnchor {
  x: number
  y: number
  width: number
  height: number
}
