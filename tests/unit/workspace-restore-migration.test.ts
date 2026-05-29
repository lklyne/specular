import { describe, expect, it } from 'vitest'
import type { WorkspaceSnapshot } from '../../src/shared/types'
import { migrateSnapshotEntityOrderForRestore } from '../../src/main/runtime/workspace-restore-migration'

function snapshot(input: Partial<WorkspaceSnapshot>): WorkspaceSnapshot {
  return {
    zoom: 1,
    pan: { x: 0, y: 0 },
    pages: [],
    entities: {},
    entityOrder: [],
    selectedPageIndex: null,
    selectedPageId: null,
    selectedPageIds: [],
    leftSidebarOpen: true,
    devtoolsOpen: false,
    devtoolsPanelTab: 'elements',
    devtoolsWidth: 400,
    browserTabMode: 'canvas',
    groups: [],
    edges: [],
    ...input,
  }
}

describe('workspace restore stack-order migration', () => {
  it('normalizes scattered group members to the frontmost member index', () => {
    const restored = snapshot({
      entityOrder: ['a', 'x', 'group', 'b', 'y'],
      entities: {
        a: {
          kind: 'text',
          id: 'a',
          text: 'A',
          color: '3',
          textStyle: 'sticky',
          widthMode: 'fixed',
          canvasX: 0,
          canvasY: 0,
          width: 100,
          height: 100,
        },
        b: {
          kind: 'text',
          id: 'b',
          text: 'B',
          color: '3',
          textStyle: 'sticky',
          widthMode: 'fixed',
          canvasX: 0,
          canvasY: 0,
          width: 100,
          height: 100,
        },
        group: {
          kind: 'group',
          id: 'group',
          label: 'Group',
          canvasX: 0,
          canvasY: 0,
          width: 300,
          height: 200,
          layoutMode: 'freeform',
          managedLayout: false,
        },
      },
      groups: [{
        id: 'group',
        kind: 'group',
        label: 'Group',
        canvasX: 0,
        canvasY: 0,
        width: 300,
        height: 200,
        layoutMode: 'freeform',
        managedLayout: false,
        entityIds: ['a', 'b'],
      }],
    })

    expect(migrateSnapshotEntityOrderForRestore(restored)).toBe(true)
    expect(restored.entityOrder).toEqual(['x', 'a', 'b', 'group', 'y'])
  })

  it('is idempotent for nested group runs', () => {
    const restored = snapshot({
      entityOrder: ['a', 'outer', 'x', 'b', 'inner', 'y'],
      entities: {
        a: {
          kind: 'shape',
          id: 'a',
          shapeKind: 'rectangle',
          text: '',
          canvasX: 0,
          canvasY: 0,
          width: 100,
          height: 100,
        },
        b: {
          kind: 'shape',
          id: 'b',
          shapeKind: 'rectangle',
          text: '',
          canvasX: 0,
          canvasY: 0,
          width: 100,
          height: 100,
        },
        inner: {
          kind: 'group',
          id: 'inner',
          label: 'Inner',
          canvasX: 0,
          canvasY: 0,
          width: 100,
          height: 100,
          parentGroupId: 'outer',
          layoutMode: 'freeform',
          managedLayout: false,
        },
        outer: {
          kind: 'group',
          id: 'outer',
          label: 'Outer',
          canvasX: 0,
          canvasY: 0,
          width: 300,
          height: 200,
          layoutMode: 'freeform',
          managedLayout: false,
        },
      },
      groups: [
        {
          id: 'outer',
          kind: 'group',
          label: 'Outer',
          canvasX: 0,
          canvasY: 0,
          width: 300,
          height: 200,
          layoutMode: 'freeform',
          managedLayout: false,
          entityIds: ['a', 'inner'],
        },
        {
          id: 'inner',
          kind: 'group',
          label: 'Inner',
          canvasX: 0,
          canvasY: 0,
          width: 100,
          height: 100,
          parentGroupId: 'outer',
          layoutMode: 'freeform',
          managedLayout: false,
          entityIds: ['b'],
        },
      ],
    })

    expect(migrateSnapshotEntityOrderForRestore(restored)).toBe(true)
    expect(restored.entityOrder).toEqual(['x', 'a', 'b', 'inner', 'outer', 'y'])
    expect(migrateSnapshotEntityOrderForRestore(restored)).toBe(false)
    expect(restored.entityOrder).toEqual(['x', 'a', 'b', 'inner', 'outer', 'y'])
  })
})
