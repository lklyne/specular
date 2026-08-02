/**
 * Tab routes — the transport behind `specular tab`.
 *
 * `GET /tabs` names every canvas in the workspace and marks the one the user
 * is looking at. `POST /tabs` creates a canvas without stealing focus, and
 * `POST /tabs/delete` removes one the same way — an agent that can create a
 * canvas can tidy it up again. `POST /tabs/switch` is the one verb that does
 * move focus.
 */

import type { Route } from './types'
import { spaceTabIdentity } from '../runtime/space-tabs'
import {
  createBackgroundSpaceTab,
  deleteSpaceTab,
  setActiveSpaceTab,
} from '../runtime/space-tab-operations'
import { resolveSpaceTabRef } from '../runtime/space-tab-refs'
import { spaceTabs } from '../runtime/space-model'
import { writeJson } from './http-helpers'

export const tabRoutes: Route[] = [
  {
    method: 'GET',
    pattern: '/tabs',
    async handler({ response }) {
      writeJson(response, 200, spaceTabIdentity())
    },
  },
  {
    method: 'POST',
    pattern: '/tabs',
    async handler({ response, body }) {
      const { name } = body as { name?: string }
      const result = createBackgroundSpaceTab(name ?? '')
      if (!result.ok) {
        writeJson(response, 400, { error: result.error })
        return
      }
      writeJson(response, 200, { id: result.id, name: (name ?? '').trim(), activated: false })
    },
  },
  {
    method: 'POST',
    pattern: '/tabs/delete',
    async handler({ response, body }) {
      const { ref } = body as { ref?: string }
      const resolved = resolveSpaceTabRef(ref ?? '')
      if (!resolved.ok) {
        writeJson(response, 400, { error: resolved.error })
        return
      }
      const { id, name } = resolved.tab
      // The last tab cannot be removed — it resets to an empty default canvas
      // instead, so say which happened rather than reporting a bare success.
      const wasLast = spaceTabs.length === 1
      if (!deleteSpaceTab(id)) {
        writeJson(response, 400, { error: `could not delete tab '${name}'` })
        return
      }
      writeJson(response, 200, {
        deleted: { id, name },
        reset: wasLast,
        activeTab: spaceTabIdentity().activeTab,
      })
    },
  },
  {
    method: 'POST',
    pattern: '/tabs/switch',
    async handler({ response, body }) {
      const { ref } = body as { ref?: string }
      const resolved = resolveSpaceTabRef(ref ?? '')
      if (!resolved.ok) {
        writeJson(response, 400, { error: resolved.error })
        return
      }
      setActiveSpaceTab(resolved.tab.id)
      writeJson(response, 200, { activeTab: { id: resolved.tab.id, name: resolved.tab.name } })
    },
  },
]
