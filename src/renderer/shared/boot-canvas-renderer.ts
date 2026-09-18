import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { CanvasLayoutBootstrapData } from '../../shared/types'
import { installFocusModality } from './focusModality'
import { installRendererErrorReporter } from './install-error-reporter'
import { runtimeStore } from './runtime-store'
import { connectRuntimeStore } from './runtime-store-feed'

/**
 * Shared boot sequence for the three canvas-surface renderers (canvas-bg,
 * above-view, agent-layer). They all hang off the same preload bridge, so
 * they all subscribe to the runtime store feed, seed the store from the
 * bootstrap snapshot, and apply the theme before mounting React.
 *
 * `errorReporterLabel` opts a window into the renderer error reporter; only
 * the primary surface needs it, overlays share its process-death signal.
 */
export async function bootCanvasRenderer(options: { errorReporterLabel?: string } = {}): Promise<{
  api: CanvasBgElectronAPI
  initialData: CanvasLayoutBootstrapData
}> {
  installFocusModality()
  if (options.errorReporterLabel) installRendererErrorReporter(options.errorReporterLabel)

  const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI
  connectRuntimeStore(api)

  const initialData = await api.getInitialData()
  runtimeStore.applySnapshot(initialData.layoutData)
  document.documentElement.classList.toggle('dark', initialData.theme.isDark)
  return { api, initialData }
}
