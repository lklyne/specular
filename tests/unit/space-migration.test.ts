/**
 * migrateSpace (ADR 0033 §3) — copy-verify-delete between two space folders.
 * Pure Node module, no Electron, so this is a plain unit test.
 *
 * Mutation-verified by: making the catch block in `migrateSpace`
 * (src/main/runtime/space-migration.ts) swallow the error instead of
 * rethrowing — "leaves originals in place when a copy cannot be verified"
 * fails because the function no longer throws and falls through to delete
 * the (partially copied) originals anyway.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { hasCanvasFiles, migrateSpace } from '../../src/main/runtime/space-migration'

let fromDir: string
let toDir: string

beforeEach(() => {
  fromDir = mkdtempSync(join(tmpdir(), 'specular-migrate-from-'))
  toDir = mkdtempSync(join(tmpdir(), 'specular-migrate-to-'))
})

afterEach(() => {
  rmSync(fromDir, { recursive: true, force: true })
  rmSync(toDir, { recursive: true, force: true })
})

describe('migrateSpace', () => {
  it('copies .canvas files, root .md notes, assets/, and .specular/, then removes the originals', () => {
    writeFileSync(join(fromDir, 'Canvas 1-abcd.canvas'), '{"nodes":[]}')
    writeFileSync(join(fromDir, 'My Note.md'), '# hello')
    mkdirSync(join(fromDir, 'assets'), { recursive: true })
    writeFileSync(join(fromDir, 'assets', 'image.png'), Buffer.from([1, 2, 3]))
    mkdirSync(join(fromDir, '.specular'), { recursive: true })
    writeFileSync(join(fromDir, '.specular', 'workspace-meta.json'), '{}')

    migrateSpace(fromDir, toDir)

    expect(readFileSync(join(toDir, 'Canvas 1-abcd.canvas'), 'utf8')).toBe('{"nodes":[]}')
    expect(readFileSync(join(toDir, 'My Note.md'), 'utf8')).toBe('# hello')
    expect(readFileSync(join(toDir, 'assets', 'image.png'))).toEqual(Buffer.from([1, 2, 3]))
    expect(readFileSync(join(toDir, '.specular', 'workspace-meta.json'), 'utf8')).toBe('{}')

    expect(existsSync(join(fromDir, 'Canvas 1-abcd.canvas'))).toBe(false)
    expect(existsSync(join(fromDir, 'My Note.md'))).toBe(false)
    expect(existsSync(join(fromDir, 'assets'))).toBe(false)
    expect(existsSync(join(fromDir, '.specular'))).toBe(false)
  })

  it('does nothing when the source space has no canvas content', () => {
    // An empty source is a no-op, not an error — nothing to copy or verify.
    expect(() => migrateSpace(fromDir, toDir)).not.toThrow()
    expect(existsSync(join(toDir, 'assets'))).toBe(false)
  })

  it('leaves originals in place when a copy cannot be verified', () => {
    writeFileSync(join(fromDir, 'Canvas 1-abcd.canvas'), '{"nodes":[]}')
    mkdirSync(join(fromDir, 'assets'), { recursive: true })
    writeFileSync(join(fromDir, 'assets', 'image.png'), Buffer.from([1, 2, 3]))

    // Force the copy of assets/image.png to fail verification: pre-create a
    // directory at its destination path so the copy can't land a file there.
    mkdirSync(join(toDir, 'assets', 'image.png'), { recursive: true })

    expect(() => migrateSpace(fromDir, toDir)).toThrow(/Failed to migrate space/)

    // Deletion only starts after every file is copied and verified, so even
    // the canvas file that copied fine before the failing asset stays put.
    expect(existsSync(join(fromDir, 'Canvas 1-abcd.canvas'))).toBe(true)
    expect(existsSync(join(fromDir, 'assets', 'image.png'))).toBe(true)
  })

  it('hasCanvasFiles distinguishes an empty destination from a populated one', () => {
    expect(hasCanvasFiles(fromDir)).toBe(false)
    writeFileSync(join(fromDir, 'x.canvas'), '{}')
    expect(hasCanvasFiles(fromDir)).toBe(true)
  })
})
