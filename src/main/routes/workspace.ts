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
import { annotateSelectionRegion } from '../runtime/annotate-selection'
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
import { withTabContext } from '../runtime/workspace-tab-context'
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
    pattern: '/selection/annotate',
    async handler({ response, body }) {
      const payload = body as { entityIds?: string[]; text?: string }
      const text = payload.text?.trim()
      if (!text) {
        writeJson(response, 400, { error: 'text is required' })
        return
      }
      try {
        const annotation = await annotateSelectionRegion({
          entityIds: payload.entityIds,
          text,
        })
        writeJson(response, 200, {
          id: annotation.id,
          anchor: annotation.anchor,
          selectionEntityIds: annotation.metadata?.selectionEntityIds,
          selectionTarget: annotation.metadata?.selectionTarget,
        })
      } catch (e) {
        writeJson(response, 400, { error: e instanceof Error ? e.message : String(e) })
      }
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
  // Single-item placement, same story as the batch pre-pass below: the
  // occupied regions it avoids belong to the target canvas, not the one the
  // user is looking at. Read-only, so the context commits nothing back.
  {
    method: 'POST',
    pattern: '/layout/find-placement',
    tabScoped: true,
    async handler({ response, body, targetTab }) {
      const compute = (): unknown => findPlacement(body as PlacementRequest)
      writeJson(
        response,
        200,
        targetTab ? withTabContext(targetTab.id, compute, { commit: false }) : compute(),
      )
    },
  },
  // The placement pre-pass every write runs before `/canvas/apply`. It honors
  // `--tab` for the same reason the write does: positions computed against the
  // canvas the user happens to be looking at would land the batch on top of
  // whatever already occupies the target. Read-only, so the context commits
  // nothing back.
  {
    method: 'POST',
    pattern: '/layout/batch-placement',
    tabScoped: true,
    async handler({ response, body, targetTab }) {
      const compute = (): unknown => findBatchPlacement(body as BatchPlacementRequest)
      writeJson(
        response,
        200,
        targetTab ? withTabContext(targetTab.id, compute, { commit: false }) : compute(),
      )
    },
  },
  {
    method: 'POST',
    pattern: '/layout/apply-directive',
    tabScoped: true,
    async handler({ response, body, targetTab }) {
      const req = body as ApplyDirectiveRequest
      const err = validateLayoutDirective(req?.layout)
      if (err) {
        writeJson(response, 400, { error: err })
        return
      }
      const compute = (): unknown => applyLayoutDirective(req)
      try {
        writeJson(
          response,
          200,
          targetTab ? withTabContext(targetTab.id, compute, { commit: false }) : compute(),
        )
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
