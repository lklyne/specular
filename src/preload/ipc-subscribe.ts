import { ipcRenderer } from 'electron'
import type { ThemeData } from '../shared/types'

/**
 * Builds an `onX(callback) => unsubscribe` bridge method for a single IPC
 * channel — the boilerplate every preload's subscribe-style API repeats.
 */
export function sub<T>(channel: string): (callback: (data: T) => void) => () => void {
  return (callback) => {
    const handler = (_event: Electron.IpcRendererEvent, data: T) => callback(data)
    ipcRenderer.on(channel, handler)
    return () => ipcRenderer.removeListener(channel, handler)
  }
}

/** Every preload bridge exposes `onThemeChanged` on the same `theme-changed` channel. */
export function makeThemeSubscriber(): (callback: (data: ThemeData) => void) => () => void {
  return sub<ThemeData>('theme-changed')
}
