// Facade: re-exports page lifecycle and page IPC functions.

export {
  findPageById,
  findPageByWebContents,
  pages,
} from './runtime-context'
export { handlePageIpcResponse, requestNodeDetail } from './page-ipc'
export type { Page } from './runtime-entities'
export {
  getComponentAncestryByNodeId,
  getComponentSourceLocationByNodeId,
  handleNodeDetailResponse,
  setSelectedInspectNodeById,
} from './inspect-session'
export { createPage, removePageById } from './page-factory'

export {
  queryElementsByName,
  queryPageElements,
  takePageAgentSnapshot,
  takePageScreenshot,
  takePageSnapshot,
} from './page-queries'

export {
  setMcpConnectionStatus,
} from './runtime-core'
