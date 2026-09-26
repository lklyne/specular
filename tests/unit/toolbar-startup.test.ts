import { EventEmitter } from 'node:events'
import { expect, it, vi } from 'vitest'
import type { ToolbarElectronAPI } from '../../src/shared/electron-api/toolbar'
import { ipcChannels } from '../../src/shared/ipc-contract'

const boundary = vi.hoisted(() => ({ api: null as ToolbarElectronAPI | null }))
vi.mock('electron', () => ({
  ipcRenderer: new EventEmitter(),
  contextBridge: { exposeInMainWorld: (_name: string, api: ToolbarElectronAPI) => { boundary.api = api } },
}))
import { ipcRenderer } from 'electron'
import '../../src/preload/toolbar'

// A restored camera can be broadcast while the renderer is still awaiting
// bootstrap data. React's effect (including StrictMode remount) subscribes later.
it('shows the restored zoom even when the initial broadcast precedes React mount', () => {
  ipcRenderer.emit(ipcChannels.zoomChanged, {}, 20)
  const values: number[] = []
  const unsubscribe = boundary.api!.onZoomChanged((value) => values.push(value))
  expect(values).toEqual([20])
  ipcRenderer.emit(ipcChannels.zoomChanged, {}, 25)
  expect(values).toEqual([20, 25])
  unsubscribe()
  ipcRenderer.emit(ipcChannels.zoomChanged, {}, 30)
  expect(values).toEqual([20, 25])
  const remounted: number[] = []
  boundary.api!.onZoomChanged((value) => remounted.push(value))()
  expect(remounted).toEqual([30])
})
