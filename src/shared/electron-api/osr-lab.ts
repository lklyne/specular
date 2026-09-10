import type {
  OsrLabConfig,
  OsrLabCursorPayload,
  OsrLabKeyPayload,
  OsrLabMainStats,
  OsrLabPageInfo,
  OsrLabPointerPayload,
  OsrLabWheelPayload,
} from '../osr-lab'
import type { ThemeData } from '../types'

export interface OsrLabElectronAPI {
  /** Tears down the current page set and creates one for `config`. */
  configure: (config: OsrLabConfig) => Promise<OsrLabPageInfo[]>
  teardown: () => Promise<void>
  getStats: () => Promise<OsrLabMainStats>
  /** `capturePage()` on the offscreen webContents — the agent screenshot path. */
  capturePage: (pageId: string) => Promise<string | null>
  openDevTools: (pageId: string) => Promise<void>
  traceStart: () => Promise<void>
  traceStop: () => Promise<string | null>
  forwardPointer: (payload: OsrLabPointerPayload) => void
  forwardWheel: (payload: OsrLabWheelPayload) => void
  forwardKey: (payload: OsrLabKeyPayload) => void
  insertText: (pageId: string, text: string) => void
  setEnteredPage: (pageId: string | null) => void
  setVisiblePages: (pageIds: string[]) => void
  onPagesChanged: (callback: (pages: OsrLabPageInfo[]) => void) => () => void
  onCursorChanged: (callback: (payload: OsrLabCursorPayload) => void) => () => void
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
}
