import type { IncomingMessage, ServerResponse } from 'http'
import type { PersistedWorkspaceTab } from '../../shared/types'

export interface RouteContext {
  request: IncomingMessage
  response: ServerResponse
  url: string
  body: unknown
  params: Record<string, string>
  /**
   * The tab this request targets, when the caller passed `--tab` (issue #360
   * §3). Resolved once in the server from the `x-specular-tab` header, so
   * every verb agrees on what a ref means. Absent means the active tab.
   */
  targetTab?: PersistedWorkspaceTab
}

export type RouteHandler = (ctx: RouteContext) => Promise<void>

export interface Route {
  method: string
  pattern: string | RegExp
  handler: RouteHandler
  /**
   * Whether this route honors `--tab`. The server refuses the header on any
   * route without it rather than letting the flag be silently ignored — a
   * write that lands on the wrong canvas with no signal is the whole bug
   * issue #360 exists to kill.
   */
  tabScoped?: boolean
}
