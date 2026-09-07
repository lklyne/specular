/**
 * beautiful-mermaid drags elkjs (~1.5MB) along for layout. Nothing on the
 * canvas needs it until the first diagram appears, so the renderer module is
 * split into its own chunk and loaded on demand. Once loaded, callers get a
 * synchronous path so re-renders never flash a placeholder.
 */

import type { MermaidRender } from './render-mermaid'

type RendererModule = typeof import('./render-mermaid')

let loaded: RendererModule | null = null
let loading: Promise<RendererModule> | null = null

export function loadMermaidRenderer(): Promise<RendererModule> {
  if (loaded) return Promise.resolve(loaded)
  loading ??= import('./render-mermaid').then((mod) => {
    loaded = mod
    return mod
  })
  return loading
}

/** Synchronous render when the chunk is already in; null before then. */
export function renderMermaidIfLoaded(source: string): MermaidRender | null {
  return loaded ? loaded.renderMermaidCached(source) : null
}
