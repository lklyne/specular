/**
 * Tab routes — the transport behind `specular tab`.
 *
 * `GET /tabs` names every canvas in the workspace and marks the one the user
 * is looking at. `POST /tabs` creates a canvas without stealing focus.
 * `POST /tabs/switch` is the one verb that does move focus.
 */

import type { Route } from './types'
import { workspaceTabIdentity } from '../runtime/workspace-tabs'
import { createBackgroundWorkspaceTab, setActiveWorkspaceTab } from '../runtime/workspace-tab-operations'
import { resolveWorkspaceTabRef } from '../runtime/workspace-tab-refs'
import { writeJson } from './http-helpers'

export const tabRoutes: Route[] = [
  {
    method: 'GET',
    pattern: '/tabs',
    async handler({ response }) {
      writeJson(response, 200, workspaceTabIdentity())
    },
  },
  {
    method: 'POST',
    pattern: '/tabs',
    async handler({ response, body }) {
      const { name } = body as { name?: string }
      const result = createBackgroundWorkspaceTab(name ?? '')
      if (!result.ok) {
        writeJson(response, 400, { error: result.error })
        return
      }
      writeJson(response, 200, { id: result.id, name: (name ?? '').trim(), activated: false })
    },
  },
  {
    method: 'POST',
    pattern: '/tabs/switch',
    async handler({ response, body }) {
      const { ref } = body as { ref?: string }
      const resolved = resolveWorkspaceTabRef(ref ?? '')
      if (!resolved.ok) {
        writeJson(response, 400, { error: resolved.error })
        return
      }
      setActiveWorkspaceTab(resolved.tab.id)
      writeJson(response, 200, { activeTab: { id: resolved.tab.id, name: resolved.tab.name } })
    },
  },
]
