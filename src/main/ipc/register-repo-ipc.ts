import { ipcChannels } from '../../shared/ipc-contract'
import { BrowserWindow, dialog, ipcMain } from 'electron'
import {
  bgView,
  devtoolsHeaderView,
  toolbarView,
} from '../runtime/view-refs'
import {
  bindOriginToRepo,
  connectRepo,
  disconnectRepo,
  findRepoForPath,
  listRepos,
  onChange,
  type ConnectedRepo,
} from '../runtime/dev-server-manager'
import { markDirty } from '../runtime/layout-dirty'
import { repoPickerDefaultPath } from '../runtime/picker-defaults'
import { requestLayout } from '../runtime/viewport-control'
import { getSettingsWebContents } from '../settings-window'

function broadcastRepos(repos: ConnectedRepo[]): void {
  const targets = [
    bgView?.webContents,
    devtoolsHeaderView?.webContents,
    toolbarView?.webContents,
    getSettingsWebContents(),
  ]
  for (const wc of targets) {
    try {
      wc?.send(ipcChannels.repoChanged, repos)
    } catch {
      // ignore — view may be in the middle of teardown
    }
  }
}

export function registerRepoIpc(): void {
  ipcMain.handle(ipcChannels.repoList, async (): Promise<ConnectedRepo[]> => listRepos())

  ipcMain.handle(
    ipcChannels.repoConnect,
    async (_event, payload: { absolutePath?: string }): Promise<ConnectedRepo | null> => {
      const path = payload?.absolutePath
      if (!path) return null
      return connectRepo(path)
    },
  )

  ipcMain.handle(ipcChannels.repoConnectViaPicker, async (event): Promise<ConnectedRepo | null> => {
    const win = BrowserWindow.fromWebContents(event.sender) ?? undefined
    const dialogOpts: Electron.OpenDialogOptions = {
      title: 'Connect a Vite repo',
      properties: ['openDirectory'],
      defaultPath: repoPickerDefaultPath(),
    }
    const result = win
      ? await dialog.showOpenDialog(win, dialogOpts)
      : await dialog.showOpenDialog(dialogOpts)
    if (result.canceled || result.filePaths.length === 0) return null
    return connectRepo(result.filePaths[0])
  })

  ipcMain.handle(
    ipcChannels.repoDisconnect,
    async (_event, payload: { id?: string }): Promise<void> => {
      if (payload?.id) await disconnectRepo(payload.id)
    },
  )

  ipcMain.handle(
    ipcChannels.repoBindOrigin,
    async (
      _event,
      payload: { repoId?: string; origin?: string },
    ): Promise<ConnectedRepo | null> => {
      const repoId = payload?.repoId
      const origin = payload?.origin?.trim()
      if (!repoId || !origin) return null
      return bindOriginToRepo(repoId, origin)
    },
  )

  ipcMain.handle(
    ipcChannels.repoFindForPath,
    async (_event, payload: { absolutePath?: string }): Promise<ConnectedRepo | null> => {
      const path = payload?.absolutePath
      if (!path) return null
      return findRepoForPath(path)
    },
  )

  onChange((repos) => {
    broadcastRepos(repos)
    // Component file entities derive `componentHasRepo` from the current
    // repo set. When that set changes, re-broadcast the canvas scene so
    // the renderer can drop or restore the placeholder for affected
    // entities, and lay out so the new component WCV (if any) gets
    // sized.
    markDirty('canvas')
    requestLayout()
  })
}
