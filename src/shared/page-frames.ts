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
  /** CSS viewport size of the page when this frame was painted. */
  cssWidth: number
  cssHeight: number
  /** The rect Electron reported dirty for this paint, in device pixels. It
   *  never carries a popup's position: a popup's own paints are popup-local. */
  dirtyRect: { x: number; y: number; width: number; height: number }
  frameCount: number | null
}

/**
 * Posted by the canvas-bg preload to the page world with a transferred
 * `bitmap: ImageBitmap` attached.
 */
export interface PageFrameMessage {
  source: 'page-frame'
  kind: 'frame'
  meta: PageFrameMeta
}

/** A focused element's rect in page CSS px — the point a popup widget hangs from. */
export interface PagePopupAnchor {
  x: number
  y: number
  width: number
  height: number
}
