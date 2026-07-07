import type { Route } from './types'
import type {
  ApplyDirectiveRequest,
  ApplyTaskLayoutRequest,
  BatchLayoutMode,
  BatchPlacementRequest,
  CanvasEntityKind,
  SpacingToken,
  LayoutComponentStatesRequest,
  PlacementRequest,
} from '../../shared/types'
import { validateLayoutDirective } from '../../shared/layout-directive'
import { getSelectionState } from '../workspace-entities'
import { arrangeEntities } from '../runtime/document-commands'
import { selectedEntityIds as currentSelectionIds } from '../ui-state'
import { applyLayoutDirective, findBatchPlacement, findPlacement } from '../workspace-placement'
import {
  applyTaskLayout,
  layoutComponentStates,
} from '../workspace-layout-tasks'
import { getLeftSidebarData } from '../runtime/canvas-layout-data'
import {
  enterGroup as enterSelectionGroup,
  selectEntities as selectSelectionEntities,
  selectEntity as selectSelectionEntity,
  selectGroup as selectSelectionGroup,
  selectNone as clearSelection,
  selectPageById as selectSelectionPageById,
} from '../runtime/selection-controller'
import { pageSelectionOverlayStates } from '../runtime/overlay-manager'
import { writeJson } from './http-helpers'

export const workspaceRoutes: Route[] = [
  {
    method: 'GET',
    pattern: '/sidebar',
    async handler({ response }) {
      writeJson(response, 200, getLeftSidebarData())
    },
  },
  {
    method: 'GET',
    pattern: '/selection',
    async handler({ response }) {
      writeJson(response, 200, getSelectionState())
    },
  },
  {
    method: 'GET',
    pattern: '/selection/overlay-state',
    async handler({ response }) {
      writeJson(response, 200, { pages: pageSelectionOverlayStates() })
    },
  },
  {
    method: 'POST',
    pattern: '/selection/arrange',
    async handler({ response, body }) {
      const payload = body as {
        mode?: BatchLayoutMode
        entityIds?: string[]
        gap?: number | SpacingToken
        cols?: number
      }
      const err = validateLayoutDirective({
        kind: payload.mode as BatchLayoutMode,
        gap: payload.gap,
        cols: payload.cols,
      })
      if (err) {
        writeJson(response, 400, { error: err })
        return
      }
      const entityIds = payload.entityIds ?? currentSelectionIds()
      const changed = arrangeEntities(entityIds, payload.mode as BatchLayoutMode, {
        gap: payload.gap,
        cols: payload.cols,
      })
      writeJson(response, 200, { changed })
    },
  },
  {
    method: 'POST',
    pattern: '/selection/deselect',
    async handler({ response }) {
      clearSelection()
      writeJson(response, 200, getSelectionState())
    },
  },
  {
    method: 'POST',
    pattern: '/selection/select-page',
    async handler({ response, body }) {
      const payload = body as { pageId?: string }
      if (!payload.pageId) {
        writeJson(response, 400, { error: 'pageId is required' })
        return
      }
      writeJson(response, 200, {
        ok: selectSelectionPageById(payload.pageId),
        selection: getSelectionState(),
      })
    },
  },
  {
    method: 'POST',
    pattern: '/selection/select-entity',
    async handler({ response, body }) {
      const payload = body as { entityId?: string; entityKind?: CanvasEntityKind }
      if (!payload.entityId || !payload.entityKind) {
        writeJson(response, 400, { error: 'entityId and entityKind are required' })
        return
      }
      writeJson(response, 200, {
        ok: selectSelectionEntity(payload.entityId, payload.entityKind),
        selection: getSelectionState(),
      })
    },
  },
  {
    method: 'POST',
    pattern: '/selection/select-entities',
    async handler({ response, body }) {
      const payload = body as { entityIds?: string[] }
      const entityIds = payload.entityIds ?? []
      writeJson(response, 200, {
        ok: selectSelectionEntities(entityIds),
        selection: getSelectionState(),
      })
    },
  },
  {
    method: 'POST',
    pattern: '/selection/select-group',
    async handler({ response, body }) {
      const payload = body as { groupId?: string }
      if (!payload.groupId) {
        writeJson(response, 400, { error: 'groupId is required' })
        return
      }
      writeJson(response, 200, {
        ok: selectSelectionGroup(payload.groupId),
        selection: getSelectionState(),
      })
    },
  },
  {
    method: 'POST',
    pattern: '/selection/enter-group',
    async handler({ response, body }) {
      const payload = body as { groupId?: string }
      if (!payload.groupId) {
        writeJson(response, 400, { error: 'groupId is required' })
        return
      }
      writeJson(response, 200, {
        ok: enterSelectionGroup(payload.groupId),
        selection: getSelectionState(),
      })
    },
  },
  {
    method: 'POST',
    pattern: '/layout/find-placement',
    async handler({ response, body }) {
      writeJson(response, 200, findPlacement(body as PlacementRequest))
    },
  },
  {
    method: 'POST',
    pattern: '/layout/batch-placement',
    async handler({ response, body }) {
      writeJson(response, 200, findBatchPlacement(body as BatchPlacementRequest))
    },
  },
  {
    method: 'POST',
    pattern: '/layout/apply-directive',
    async handler({ response, body }) {
      const req = body as ApplyDirectiveRequest
      const err = validateLayoutDirective(req?.layout)
      if (err) {
        writeJson(response, 400, { error: err })
        return
      }
      try {
        writeJson(response, 200, applyLayoutDirective(req))
      } catch (e) {
        writeJson(response, 400, { error: e instanceof Error ? e.message : String(e) })
      }
    },
  },
  {
    method: 'POST',
    pattern: '/tasks/apply',
    async handler({ response, body }) {
      writeJson(response, 200, applyTaskLayout(body as ApplyTaskLayoutRequest))
    },
  },
  {
    method: 'POST',
    pattern: '/tasks/component-states',
    async handler({ response, body }) {
      writeJson(response, 200, layoutComponentStates(body as LayoutComponentStatesRequest))
    },
  },
]
