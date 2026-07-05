import { describe, expect, it } from 'vitest'
import {
  beginEdgeDrag,
  buildEdgeDragPath,
  cancelEdgeDrag,
  commitEdgeDrag,
  EDGE_DRAG_IDLE,
  edgeDragOrigin,
  updateEdgeDragCursor,
} from '../../src/shared/edge-drag-controller'
import type { CanvasSceneEntity, WorkspaceEdge } from '../../src/shared/types'

function page(id: string, x: number, y: number): CanvasSceneEntity {
  return {
    id,
    kind: 'page',
    canvasX: x,
    canvasY: y,
    width: 200,
    height: 100,
    screenX: x,
    screenY: y,
    screenWidth: 200,
    screenHeight: 100,
    presetIndex: 0,
    rendererTag: 'web',
  } as CanvasSceneEntity
}

function entityMap(...entities: CanvasSceneEntity[]): Map<string, CanvasSceneEntity> {
  return new Map(entities.map((e) => [e.id, e]))
}

describe('edge-drag-controller', () => {
  describe('beginEdgeDrag', () => {
    it('starts in create mode when the anchor has no existing edge', () => {
      const e = page('a', 0, 0)
      const state = beginEdgeDrag('a', 'right', 250, 50, [], entityMap(e))
      expect(state.kind).toBe('create')
      if (state.kind === 'create') {
        expect(state.fromEntityId).toBe('a')
        expect(state.fromSide).toBe('right')
        expect(state.snap).toBeNull()
      }
    })

    it('starts in edit mode when the anchor hosts an existing edge', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      const edges: WorkspaceEdge[] = [
        { id: 'e1', fromEntityId: 'a', toEntityId: 'b', fromSide: 'right', toSide: 'left' } as WorkspaceEdge,
      ]
      const state = beginEdgeDrag('b', 'left', 410, 50, edges, entityMap(a, b))
      expect(state.kind).toBe('edit')
      if (state.kind === 'edit') {
        expect(state.edgeId).toBe('e1')
        expect(state.movingEnd).toBe('to')
        expect(state.fixedEntityId).toBe('a')
        expect(state.fixedSide).toBe('right')
      }
    })

    it('uses auto-sides when the existing edge does not specify sides', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      const edges: WorkspaceEdge[] = [
        { id: 'e1', fromEntityId: 'a', toEntityId: 'b' } as WorkspaceEdge,
      ]
      // Auto-sides chooses right→left for two horizontally-arranged pages.
      const state = beginEdgeDrag('a', 'right', 200, 50, edges, entityMap(a, b))
      expect(state.kind).toBe('edit')
    })
  })

  describe('updateEdgeDragCursor', () => {
    it('snaps to a target anchor when within snap distance', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      let state = beginEdgeDrag('a', 'right', 250, 50, [], entityMap(a, b))
      state = updateEdgeDragCursor(state, 392, 50, entityMap(a, b), 1)
      // b's left anchor is at (392, 50) given DOT_OFFSET=8.
      if (state.kind !== 'create') throw new Error('expected create state')
      expect(state.snap?.entityId).toBe('b')
      expect(state.snap?.side).toBe('left')
    })

    it('clears snap when the cursor leaves the snap radius', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      let state = beginEdgeDrag('a', 'right', 250, 50, [], entityMap(a, b))
      state = updateEdgeDragCursor(state, 250, 250, entityMap(a, b), 1)
      if (state.kind !== 'create') throw new Error('expected create state')
      expect(state.snap).toBeNull()
    })

    it('never snaps to the source entity', () => {
      const a = page('a', 0, 0)
      let state = beginEdgeDrag('a', 'right', 208, 50, [], entityMap(a))
      // Cursor lands very close to a's right anchor — but a is the source.
      state = updateEdgeDragCursor(state, 208, 50, entityMap(a), 1)
      if (state.kind !== 'create') throw new Error('expected create state')
      expect(state.snap).toBeNull()
    })

    it('idle state passes through unchanged', () => {
      const out = updateEdgeDragCursor(EDGE_DRAG_IDLE, 0, 0, new Map(), 1)
      expect(out).toBe(EDGE_DRAG_IDLE)
    })
  })

  describe('commitEdgeDrag', () => {
    it('create + snap → create-edge outcome', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      let state = beginEdgeDrag('a', 'right', 250, 50, [], entityMap(a, b))
      state = updateEdgeDragCursor(state, 392, 50, entityMap(a, b), 1)
      const outcome = commitEdgeDrag(state)
      expect(outcome).toEqual({
        kind: 'create-edge',
        fromEntityId: 'a',
        fromSide: 'right',
        toEntityId: 'b',
        toSide: 'left',
      })
    })

    it('create without snap → noop', () => {
      const a = page('a', 0, 0)
      const state = beginEdgeDrag('a', 'right', 250, 250, [], entityMap(a))
      expect(commitEdgeDrag(state)).toEqual({ kind: 'noop' })
    })

    it('edit + snap → edit-edge outcome', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      const c = page('c', 0, 300)
      const edges: WorkspaceEdge[] = [
        { id: 'e1', fromEntityId: 'a', toEntityId: 'b', fromSide: 'right', toSide: 'left' } as WorkspaceEdge,
      ]
      let state = beginEdgeDrag('b', 'left', 410, 50, edges, entityMap(a, b, c))
      state = updateEdgeDragCursor(state, 100, 292, entityMap(a, b, c), 1)
      const outcome = commitEdgeDrag(state)
      expect(outcome).toEqual({
        kind: 'edit-edge',
        edgeId: 'e1',
        movingEnd: 'to',
        targetEntityId: 'c',
        targetSide: 'top',
      })
    })

    it('edit without snap → discard-edge', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      const edges: WorkspaceEdge[] = [
        { id: 'e1', fromEntityId: 'a', toEntityId: 'b', fromSide: 'right', toSide: 'left' } as WorkspaceEdge,
      ]
      let state = beginEdgeDrag('b', 'left', 410, 50, edges, entityMap(a, b))
      state = updateEdgeDragCursor(state, 1000, 1000, entityMap(a, b), 1)
      expect(commitEdgeDrag(state)).toEqual({ kind: 'discard-edge', edgeId: 'e1' })
    })

    it('idle → noop', () => {
      expect(commitEdgeDrag(EDGE_DRAG_IDLE)).toEqual({ kind: 'noop' })
    })
  })

  describe('cancelEdgeDrag', () => {
    it('edit → discard-edge', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      const edges: WorkspaceEdge[] = [
        { id: 'e1', fromEntityId: 'a', toEntityId: 'b', fromSide: 'right', toSide: 'left' } as WorkspaceEdge,
      ]
      const state = beginEdgeDrag('b', 'left', 410, 50, edges, entityMap(a, b))
      expect(cancelEdgeDrag(state)).toEqual({ kind: 'discard-edge', edgeId: 'e1' })
    })

    it('create → noop (nothing to discard)', () => {
      const a = page('a', 0, 0)
      const state = beginEdgeDrag('a', 'right', 250, 50, [], entityMap(a))
      expect(cancelEdgeDrag(state)).toEqual({ kind: 'noop' })
    })
  })

  describe('edgeDragOrigin', () => {
    it('create drag → the grabbed anchor', () => {
      const a = page('a', 0, 0)
      const state = beginEdgeDrag('a', 'right', 250, 50, [], entityMap(a))
      expect(edgeDragOrigin(state)).toEqual({ entityId: 'a', side: 'right' })
    })

    it('edit drag moving the to-end → the fixed from-endpoint, not the grabbed anchor', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      const edges: WorkspaceEdge[] = [
        { id: 'e1', fromEntityId: 'a', toEntityId: 'b', fromSide: 'right', toSide: 'left' } as WorkspaceEdge,
      ]
      const state = beginEdgeDrag('b', 'left', 410, 50, edges, entityMap(a, b))
      expect(edgeDragOrigin(state)).toEqual({ entityId: 'a', side: 'right' })
    })

    it('edit drag moving the from-end → the fixed to-endpoint', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      const edges: WorkspaceEdge[] = [
        { id: 'e1', fromEntityId: 'a', toEntityId: 'b', fromSide: 'right', toSide: 'left' } as WorkspaceEdge,
      ]
      const state = beginEdgeDrag('a', 'right', 208, 50, edges, entityMap(a, b))
      expect(edgeDragOrigin(state)).toEqual({ entityId: 'b', side: 'left' })
    })

    it('idle → null', () => {
      expect(edgeDragOrigin(EDGE_DRAG_IDLE)).toBeNull()
    })
  })

  describe('buildEdgeDragPath', () => {
    it('produces an SVG path for create state with snap', () => {
      const a = page('a', 0, 0)
      const b = page('b', 400, 0)
      let state = beginEdgeDrag('a', 'right', 250, 50, [], entityMap(a, b))
      state = updateEdgeDragCursor(state, 392, 50, entityMap(a, b), 1)
      const path = buildEdgeDragPath(state, entityMap(a, b), 1)
      expect(path).not.toBeNull()
      expect(path!.d).toMatch(/^M [\d.]+ [\d.]+ C /)
      expect(path!.from.side).toBe('right')
      expect(path!.to.side).toBe('left')
    })

    it('renders to cursor when no snap target', () => {
      const a = page('a', 0, 0)
      let state = beginEdgeDrag('a', 'right', 250, 50, [], entityMap(a))
      state = updateEdgeDragCursor(state, 250, 250, entityMap(a), 1)
      const path = buildEdgeDragPath(state, entityMap(a), 1)
      expect(path).not.toBeNull()
      expect(path!.to.x).toBe(250)
      expect(path!.to.y).toBe(250)
    })

    it('idle state produces null', () => {
      expect(buildEdgeDragPath(EDGE_DRAG_IDLE, new Map(), 1)).toBeNull()
    })
  })
})
