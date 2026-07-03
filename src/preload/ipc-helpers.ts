import { ipcRenderer } from 'electron'

/**
 * Subscribe to an IPC channel, forwarding the payload (event arg stripped) to
 * `callback`. Returns an unsubscribe fn. Collapses the copy-pasted
 * on/removeListener closure every preload bridge otherwise repeats per channel.
 */
export function on<T = void>(channel: string) {
  return (callback: (payload: T) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, payload: T) => callback(payload)
    ipcRenderer.on(channel, handler)
    return () => ipcRenderer.removeListener(channel, handler)
  }
}
