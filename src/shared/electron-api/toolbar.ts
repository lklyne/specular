import type { Tool } from '../tool'
import type {
  AgentPresenceCursor,
  AppThemeMode,
  ConnectedRepo,
  ThemeBootstrapData,
  ThemeData,
  ToolbarSelectionData,
} from '../types'
import type {
  ShareLinkInfo,
  ShareResult,
  ShareScope,
  ShareStateData,
} from '../share'

export interface ToolbarElectronAPI {
  zoomIn: () => void
  zoomOut: () => void
  zoomReset: () => void
  zoomSet: (level: number) => void
  setTool: (tool: Tool) => void
  reloadApp: () => void
  setThemeMode: (mode: AppThemeMode) => void
  getInitialData: () => Promise<ThemeBootstrapData>
  toggleLeftSidebar: () => void
  toggleDevTools: () => void
  dropdownOpen: () => void
  dropdownClose: () => void
  tooltipOpen: () => void
  tooltipClose: () => void
  setTextEditing: (active: boolean) => void
  onZoomChanged: (callback: (value: number) => void) => () => void
  onSelectionChanged: (callback: (data: ToolbarSelectionData) => void) => () => void
  onLeftSidebarChanged: (callback: (open: boolean) => void) => () => void
  onDevtoolsChanged: (callback: (open: boolean) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
  onAgentPresenceChanged: (callback: (cursors: AgentPresenceCursor[]) => void) => () => void
  repoConnectViaPicker: () => Promise<ConnectedRepo | null>
  repoDisconnect: (id: string) => Promise<void>
  shareState: () => Promise<ShareStateData>
  sharePublish: () => Promise<ShareResult<ShareStateData>>
  shareCopyLink: (scope: ShareScope) => Promise<ShareResult<{ url: string }>>
  shareJoin: (link: string) => Promise<ShareResult<ShareStateData>>
  shareListLinks: () => Promise<ShareResult<ShareLinkInfo[]>>
  shareResetLink: (grantId: string) => Promise<ShareResult<ShareLinkInfo>>
  shareRevokeLink: (grantId: string) => Promise<ShareResult<{ revoked: string }>>
}
