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
  /**
   * Popup widgets only: the dirty rect Electron passed to the paint event, in
   * CSS px, as a best-effort popup position. null for frames.
   */
  popupRect: { x: number; y: number; width: number; height: number } | null
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
