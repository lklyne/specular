/**
 * Where a fix runs: the repo bound to the annotation's page origin, or the
 * folder holding the file entity the selection targets.
 *
 * Mutation-verified by making resolveFixTarget check the selection target
 * before the origin binding (the page case then resolves to 'space-folder')
 * and by dropping the `filePath` guard (the file case then resolves with an
 * empty cwd) — each broke the expected case below.
 */

import { describe, expect, it } from 'vitest'
import {
  fixTargetKey,
  resolveFixTarget,
  type OriginBindingLookup,
} from '../../src/main/agent-fix/fix-target'
import type { Annotation } from '../../src/shared/types'

const boundRepo: OriginBindingLookup = (origin) =>
  origin === 'http://localhost:4321' ? { repoPath: '/Users/x/dev/site', autoFix: true } : null

const noBinding: OriginBindingLookup = () => null

function annotation(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: 'ann-1',
    anchor: { type: 'canvas', canvasX: 0, canvasY: 0 },
    author: 'user',
    text: 'Tighten this up',
    status: 'pending',
    replies: [],
    createdAt: '2026-01-01T00:00:00.000Z',
    ...overrides,
  }
}

const fileTargetMetadata = {
  selectionEntityIds: ['file-1'],
  selectionTarget: {
    entityId: 'file-1',
    kind: 'file' as const,
    filePath: '/Users/x/space/hero.png',
  },
}

describe('resolveFixTarget', () => {
  it('runs in the repo bound to the annotation page origin', () => {
    const target = resolveFixTarget(
      annotation({ pageAnchor: { pageId: 'page-a', pageUrl: 'http://localhost:4321/garden' } }),
      boundRepo,
    )
    expect(target).toEqual({
      kind: 'repo',
      cwd: '/Users/x/dev/site',
      origin: 'http://localhost:4321',
      autoFix: true,
    })
  })

  it('prefers the bound repo over a file target when both apply', () => {
    const target = resolveFixTarget(
      annotation({
        pageAnchor: { pageId: 'page-a', pageUrl: 'http://localhost:4321/garden' },
        metadata: fileTargetMetadata,
      }),
      boundRepo,
    )
    expect(target?.kind).toBe('repo')
  })

  it('runs in the folder holding a file target when no repo is bound', () => {
    const target = resolveFixTarget(annotation({ metadata: fileTargetMetadata }), noBinding)
    expect(target).toEqual({
      kind: 'space-folder',
      cwd: '/Users/x/space',
      filePath: '/Users/x/space/hero.png',
    })
  })

  it('falls back to the file target when the page origin has no repo', () => {
    const target = resolveFixTarget(
      annotation({
        pageAnchor: { pageId: 'page-a', pageUrl: 'http://example.com/garden' },
        metadata: fileTargetMetadata,
      }),
      noBinding,
    )
    expect(target?.kind).toBe('space-folder')
  })

  it('resolves nothing without a bound origin or a file target', () => {
    expect(resolveFixTarget(annotation(), noBinding)).toBeNull()
    // A file target whose entity had no path on disk names no folder to run in.
    expect(
      resolveFixTarget(
        annotation({
          metadata: {
            selectionEntityIds: ['file-1'],
            selectionTarget: { entityId: 'file-1', kind: 'file' },
          },
        }),
        noBinding,
      ),
    ).toBeNull()
    expect(
      resolveFixTarget(
        annotation({
          metadata: {
            selectionEntityIds: ['page-a'],
            selectionTarget: { entityId: 'page-a', kind: 'page', url: 'http://example.com/' },
          },
        }),
        noBinding,
      ),
    ).toBeNull()
  })
})

describe('fixTargetKey', () => {
  it('keys a repo run by origin and a space-folder run by its folder', () => {
    expect(
      fixTargetKey({
        kind: 'repo',
        cwd: '/Users/x/dev/site',
        origin: 'http://localhost:4321',
        autoFix: false,
      }),
    ).toBe('http://localhost:4321')
    expect(
      fixTargetKey({
        kind: 'space-folder',
        cwd: '/Users/x/space',
        filePath: '/Users/x/space/hero.png',
      }),
    ).toBe('file:///Users/x/space')
  })
})
