import { describe, expect, it } from 'vitest'
import { routePointerDoubleClick } from '../../src/shared/canvas-pointer-actions'
import type { HitTarget } from '../../src/shared/hit-test'

function target(payload: HitTarget['payload']): HitTarget {
  return {
    layer: 'body',
    region: { kind: 'rect', rect: { x: 0, y: 0, width: 1, height: 1 } },
    payload,
  }
}

describe('routePointerDoubleClick', () => {
  it('shape body → request-entity-edit', () => {
    expect(
      routePointerDoubleClick(target({ kind: 'entity-body', entityId: 's1', entityKind: 'shape' })),
    ).toEqual({ kind: 'request-entity-edit', entityId: 's1' })
  })

  it('text body → request-entity-edit', () => {
    expect(
      routePointerDoubleClick(target({ kind: 'entity-body', entityId: 't1', entityKind: 'text' })),
    ).toEqual({ kind: 'request-entity-edit', entityId: 't1' })
  })

  it('group body → enter-group', () => {
    expect(
      routePointerDoubleClick(target({ kind: 'entity-body', entityId: 'g1', entityKind: 'group' })),
    ).toEqual({ kind: 'enter-group', groupId: 'g1' })
  })

  it('group chrome → enter-group-rename', () => {
    expect(
      routePointerDoubleClick(target({ kind: 'chrome', entityId: 'g1', entityKind: 'group' })),
    ).toEqual({ kind: 'enter-group-rename', groupId: 'g1' })
  })

  it('page chrome → noop (chrome owns its own dbl-click)', () => {
    expect(
      routePointerDoubleClick(target({ kind: 'chrome', entityId: 'f1', entityKind: 'page' })),
    ).toEqual({ kind: 'noop' })
  })

  it('editable file body (markdown/wireframe/video) → request-entity-edit', () => {
    expect(
      routePointerDoubleClick(
        target({ kind: 'entity-body', entityId: 'fi1', entityKind: 'file', rendererEditable: true }),
      ),
    ).toEqual({ kind: 'request-entity-edit', entityId: 'fi1' })
  })

  it('non-editable file body (image/component) → noop (graceful degradation)', () => {
    expect(
      routePointerDoubleClick(
        target({ kind: 'entity-body', entityId: 'fi2', entityKind: 'file', rendererEditable: false }),
      ),
    ).toEqual({ kind: 'noop' })
  })

  it('file body with no rendererEditable signal (legacy / unclaimed) → noop', () => {
    expect(
      routePointerDoubleClick(target({ kind: 'entity-body', entityId: 'fi3', entityKind: 'file' })),
    ).toEqual({ kind: 'noop' })
  })

  it('page body → enter-page-interactive (#124 reliable enter path)', () => {
    expect(
      routePointerDoubleClick(target({ kind: 'page-body', entityId: 'p1' })),
    ).toEqual({ kind: 'enter-page-interactive', entityId: 'p1' })
  })

  it('background → noop', () => {
    expect(routePointerDoubleClick(target({ kind: 'background' }))).toEqual({ kind: 'noop' })
  })

})
