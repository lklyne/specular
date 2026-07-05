import { ipcChannels } from '../shared/ipc-contract'
import type { DesignSystemManifest } from '../shared/design-system-types'
import { pages } from './runtime/page-runtime'
import { bgView } from './runtime/window-shell'

let activeManifest: DesignSystemManifest | null = null

function broadcastManifest(manifest: DesignSystemManifest | null): void {
  for (const page of pages) {
    const webContents = page.pageView.webContents
    if (webContents.isDestroyed()) continue
    webContents.send(ipcChannels.setDesignSystemManifest, manifest)
  }

  if (bgView && !bgView.webContents.isDestroyed()) {
    bgView.webContents.send(ipcChannels.setDesignSystemManifest, manifest)
  }
}

export function loadManifest(manifest: DesignSystemManifest): DesignSystemManifest {
  activeManifest = manifest
  broadcastManifest(activeManifest)
  return activeManifest
}

export function getManifest(): DesignSystemManifest | null {
  return activeManifest
}

export function clearManifest(): void {
  activeManifest = null
  broadcastManifest(null)
}
