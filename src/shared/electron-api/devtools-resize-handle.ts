import type { ThemeBootstrapData, ThemeData } from '../types'

export interface DevtoolsResizeHandleElectronAPI {
  devtoolsResizeStart: (screenX: number) => void
  devtoolsResizeMove: (screenX: number) => void
  devtoolsResizeEnd: () => void
  getInitialData: () => Promise<ThemeBootstrapData>
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}
