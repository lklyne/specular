import type { ToolbarElectronAPI } from '../../shared/electron-api/toolbar'

export const toolbarApi = (window as unknown as { electronAPI: ToolbarElectronAPI }).electronAPI
