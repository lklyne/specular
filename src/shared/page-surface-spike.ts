/**
 * Runtime switch for the WebGPU page-surface measurement spike (throwaway;
 * see ADR 0038's territory). One `localStorage` key picks which page-surface
 * arm a session runs, read once per process so a session's behavior stays
 * fixed for its lifetime.
 */
import type { PageFrameMeta } from './page-frames'

export type PageSurfaceArm = '2d' | 'webgpu-import' | 'webgpu-blit' | 'three'

export const PAGE_SURFACE_ARM_KEY = 'specular.spike.pageSurfaceArm'

/** Anything unrecognized falls back to the shipping 2D path. */
export function parsePageSurfaceArm(value: string | null | undefined): PageSurfaceArm {
  if (value === 'webgpu-import' || value === 'webgpu-blit' || value === 'three') return value
  return '2d'
}

/**
 * The video-frame transport between the canvas-bg preload and the page
 * world, parallel to `PageFrameMessage` in `./page-frames` but carrying a
 * transferred `VideoFrame` instead of a copied `ImageBitmap`. A distinct
 * `source` keeps these messages invisible to the shipping `usePageFrames`
 * listener, and vice versa.
 */
export type PageVideoFrameMessage =
  | { source: 'page-video-frame'; kind: 'ready' }
  | { source: 'page-video-frame'; kind: 'frame'; meta: PageFrameMeta; frame: VideoFrame }
