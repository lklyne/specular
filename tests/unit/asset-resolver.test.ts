/**
 * Asset resolver (ADR 0018 §3, cloud-sync spike step 5).
 *
 * The resolver is the single seam mapping a content-addressed `asset://` file
 * reference to a location — a local `assets/` path on desktop, or the sync
 * server's asset route once a canvas is published. Local must win over remote
 * so a machine that already has the bytes never round-trips over the
 * network, and an unresolvable reference (no local copy, no sync binding)
 * must report null rather than guessing.
 *
 * Mutation-verified by:
 *   - removing the `HASH_PATTERN.test(hash)` guard in `parseAssetReference` —
 *     "rejects a malformed hash" fails (a garbage hash now parses).
 *   - swapping the local/remote branch order in `resolveAssetReference` (check
 *     `syncBaseUrl` before `findLocalAsset`) — "local wins over remote" fails
 *     because it returns the remote URL even though a local file exists.
 *   - dropping the `if (!ctx.syncBaseUrl) return null` fallthrough (returning
 *     a URL built from `undefined`) — "unresolvable without a binding" fails.
 */

import { describe, expect, it } from 'vitest'
import {
  isAssetReference,
  parseAssetReference,
  resolveAssetReference,
} from '../../src/main/runtime/asset-resolver'

const HASH = 'a'.repeat(64)

describe('isAssetReference', () => {
  it('recognizes the asset:// scheme', () => {
    expect(isAssetReference(`asset://${HASH}`)).toBe(true)
    expect(isAssetReference(`asset://${HASH}.png`)).toBe(true)
  })

  it('rejects local paths and http(s) urls', () => {
    expect(isAssetReference('/tmp/x.png')).toBe(false)
    expect(isAssetReference('local-file:///tmp/x.png')).toBe(false)
    expect(isAssetReference('https://example.com/x.png')).toBe(false)
  })
})

describe('parseAssetReference', () => {
  it('parses hash and extension', () => {
    expect(parseAssetReference(`asset://${HASH}.png`)).toEqual({ hash: HASH, ext: 'png' })
  })

  it('parses a bare hash with no extension', () => {
    expect(parseAssetReference(`asset://${HASH}`)).toEqual({ hash: HASH, ext: undefined })
  })

  it('returns null for a non-asset string', () => {
    expect(parseAssetReference('/tmp/x.png')).toBeNull()
  })

  it('returns null for a malformed (non-hex or wrong-length) hash', () => {
    expect(parseAssetReference('asset://not-a-hash')).toBeNull()
    expect(parseAssetReference(`asset://${'a'.repeat(63)}`)).toBeNull()
  })

  it('returns null for a trailing dot with no extension', () => {
    expect(parseAssetReference(`asset://${HASH}.`)).toBeNull()
  })
})

describe('resolveAssetReference', () => {
  it('resolves to the local file when one exists in the assets dir', () => {
    const resolved = resolveAssetReference(`asset://${HASH}.png`, {
      syncBaseUrl: 'https://sync.example.com',
      localAssetsDir: '/workspace/assets',
      listDir: () => [`${HASH}.png`],
    })
    expect(resolved).toEqual({ kind: 'local', path: `/workspace/assets/${HASH}.png` })
  })

  it('matches a local file by hash even when the reference omits the extension', () => {
    const resolved = resolveAssetReference(`asset://${HASH}`, {
      syncBaseUrl: null,
      localAssetsDir: '/workspace/assets',
      listDir: () => [`${HASH}.jpg`],
    })
    expect(resolved).toEqual({ kind: 'local', path: `/workspace/assets/${HASH}.jpg` })
  })

  it('local wins over remote when both are available', () => {
    const resolved = resolveAssetReference(`asset://${HASH}.png`, {
      syncBaseUrl: 'https://sync.example.com',
      localAssetsDir: '/workspace/assets',
      listDir: () => [`${HASH}.png`, 'unrelated-file.txt'],
    })
    expect(resolved?.kind).toBe('local')
  })

  it('falls back to the sync server when no local copy exists', () => {
    const resolved = resolveAssetReference(`asset://${HASH}.png`, {
      syncBaseUrl: 'https://sync.example.com',
      localAssetsDir: '/workspace/assets',
      listDir: () => [],
    })
    expect(resolved).toEqual({ kind: 'remote', url: `https://sync.example.com/assets/${HASH}` })
  })

  it('trims a trailing slash off the sync base url', () => {
    const resolved = resolveAssetReference(`asset://${HASH}.png`, {
      syncBaseUrl: 'https://sync.example.com/',
      localAssetsDir: '/workspace/assets',
      listDir: () => [],
    })
    expect(resolved).toEqual({ kind: 'remote', url: `https://sync.example.com/assets/${HASH}` })
  })

  it('is unresolvable with no local copy and no sync binding', () => {
    const resolved = resolveAssetReference(`asset://${HASH}.png`, {
      syncBaseUrl: null,
      localAssetsDir: '/workspace/assets',
      listDir: () => [],
    })
    expect(resolved).toBeNull()
  })

  it('returns null for a non-asset-reference input regardless of context', () => {
    const resolved = resolveAssetReference('/tmp/x.png', {
      syncBaseUrl: 'https://sync.example.com',
      localAssetsDir: '/workspace/assets',
      listDir: () => ['x.png'],
    })
    expect(resolved).toBeNull()
  })
})
