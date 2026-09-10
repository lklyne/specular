import { contextBridge, ipcRenderer, sharedTexture } from 'electron'
import type { OsrLabElectronAPI } from '../shared/electron-api/osr-lab'
import type { OsrLabFrameMessage, OsrLabFrameMeta } from '../shared/osr-lab'
import { ipcChannels } from '../shared/ipc-contract'
import { on, send } from './ipc-helpers'

/**
 * Frames reach the page world as `window.postMessage` transfers rather than
 * through the contextBridge: an ImageBitmap cannot cross the bridge, but the
 * DOM window is shared between the isolated and main worlds and transfers
 * work on it. The preload copies each frame into an ImageBitmap it owns and
 * releases the shared texture immediately, which is the lifecycle Electron's
 * OSR documentation asks for.
 */
function postFrame(meta: OsrLabFrameMeta, bitmap: ImageBitmap, copyMs: number): void {
  const message: OsrLabFrameMessage = {
    source: 'osr-lab',
    kind: 'frame',
    meta,
    copyMs,
    receivedAt: performance.now(),
  }
  window.postMessage({ ...message, bitmap }, '*', [bitmap])
}

sharedTexture.setSharedTextureReceiver(async (data, ...args) => {
  const meta = args[0] as OsrLabFrameMeta
  const imported = data.importedSharedTexture
  const started = performance.now()
  let frame: VideoFrame | null = null
  try {
    frame = imported.getVideoFrame()
    const bitmap = await createImageBitmap(frame)
    postFrame(meta, bitmap, performance.now() - started)
  } catch (error) {
    console.error('[osr-lab] frame copy failed', error)
  } finally {
    frame?.close()
    imported.release()
  }
})

ipcRenderer.on(
  ipcChannels.osrLabFrameJpeg,
  (_event, payload: { meta: OsrLabFrameMeta; jpeg: Uint8Array }) => {
    const started = performance.now()
    void createImageBitmap(new Blob([payload.jpeg as BlobPart], { type: 'image/jpeg' }))
      .then((bitmap) => postFrame(payload.meta, bitmap, performance.now() - started))
      .catch((error: unknown) => console.error('[osr-lab] jpeg decode failed', error))
  },
)

const api: OsrLabElectronAPI = {
  configure: (config) => ipcRenderer.invoke(ipcChannels.osrLabConfigure, config),
  teardown: () => ipcRenderer.invoke(ipcChannels.osrLabTeardown),
  getStats: () => ipcRenderer.invoke(ipcChannels.osrLabStats),
  capturePage: (pageId) => ipcRenderer.invoke(ipcChannels.osrLabCapture, pageId),
  openDevTools: (pageId) => ipcRenderer.invoke(ipcChannels.osrLabOpenDevtools, pageId),
  traceStart: () => ipcRenderer.invoke(ipcChannels.osrLabTraceStart),
  traceStop: () => ipcRenderer.invoke(ipcChannels.osrLabTraceStop),
  forwardPointer: (payload) => send(ipcChannels.osrLabPointer, payload),
  forwardWheel: (payload) => send(ipcChannels.osrLabWheel, payload),
  forwardKey: (payload) => send(ipcChannels.osrLabKey, payload),
  insertText: (pageId, text) => send(ipcChannels.osrLabInsertText, { pageId, text }),
  setEnteredPage: (pageId) => send(ipcChannels.osrLabSetEntered, { pageId }),
  setVisiblePages: (pageIds) => send(ipcChannels.osrLabSetVisible, { pageIds }),
  onPagesChanged: on(ipcChannels.osrLabPagesChanged),
  onCursorChanged: on(ipcChannels.osrLabCursorChanged),
  onThemeChanged: on(ipcChannels.themeChanged),
}

contextBridge.exposeInMainWorld('electronAPI', api)
