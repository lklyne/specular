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
 * Posted across canvas-bg's shared window between its preload and page world.
 * `ready` goes up once the page world has a listener that will close every
 * frame it receives; until then the preload drops frames. `frame` comes down
 * with the page's `VideoFrame` transferred, and the receiver owns closing it.
 */
export type PageFrameMessage =
  | { source: 'page-frame'; kind: 'ready' }
  | { source: 'page-frame'; kind: 'frame'; meta: PageFrameMeta; frame: VideoFrame }

/** A focused element's rect in page CSS px — the point a popup widget hangs from. */
export interface PagePopupAnchor {
  x: number
  y: number
  width: number
  height: number
}
