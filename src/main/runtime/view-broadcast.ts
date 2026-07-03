import type { WebContents } from 'electron'
import type { IpcChannel, IpcPayload } from '../../shared/ipc-contract'
import { getDebugWebContents } from '../debug-window'
import { getSettingsWebContents } from '../settings-window'
import {
  aboveView,
  bgView,
  devtoolsHeaderView,
  devtoolsResizeHandleView,
  leftSidebarView,
  toolbarView,
} from './view-refs'

/**
 * The only role any broadcaster filters by: cursor-spline-viz debug sends reach
 * the canvas views (bgView, aboveView) plus the debug window. Theme sends reach
 * every registered target, so they pass no filter.
 */
export type ViewRole = 'debug'

type ViewSource = {
  resolve: () => WebContents | null | undefined
  roles: readonly ViewRole[]
}

// Order mirrors broadcastTheme's original send sequence; sends are independent
// so order is not load-bearing, but keeping it stable eases diffing.
const viewSources: readonly ViewSource[] = [
  { resolve: () => bgView?.webContents, roles: ['debug'] },
  { resolve: () => leftSidebarView?.webContents, roles: [] },
  { resolve: () => toolbarView?.webContents, roles: [] },
  { resolve: () => aboveView?.webContents, roles: ['debug'] },
  { resolve: () => devtoolsHeaderView?.webContents, roles: [] },
  { resolve: () => devtoolsResizeHandleView?.webContents, roles: [] },
  { resolve: () => getDebugWebContents(), roles: ['debug'] },
  { resolve: () => getSettingsWebContents(), roles: [] },
]

/**
 * Fan out one main→renderer channel to every registered view (or the subset
 * carrying `role`). Owns the single destroyed-target guard: a source that
 * resolves to null or a disposed webContents is skipped.
 */
export function broadcast<C extends IpcChannel>(
  channel: C,
  payload: IpcPayload<C>,
  role?: ViewRole,
): void {
  for (const source of viewSources) {
    if (role && !source.roles.includes(role)) continue
    const wc = source.resolve()
    if (!wc || wc.isDestroyed()) continue
    wc.send(channel, payload)
  }
}
