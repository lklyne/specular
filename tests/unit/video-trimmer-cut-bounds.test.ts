/**
 * video-trimmer.ts cuts a recording using activity segments' wall-clock
 * `startTime`/`endTime` (seconds since recording start). That only lands
 * correctly if the source file's own timeline is wall-clock-accurate — see
 * the video-recorder pacing fix (video-recorder.ts) this test suite
 * accompanies. Given a wall-clock-accurate input, this pins that the cut
 * points handed to ffmpeg are exactly the active segments' own boundaries
 * (not shifted, not the idle segments'), and that the reported durations sum
 * the segments rather than re-deriving them from something else.
 *
 * Mocks `child_process.spawn` (a process boundary, per tests/README) so no
 * real ffmpeg runs; segments are passed in directly rather than read from a
 * fixture file.
 *
 * Mutation-verified by swapping `activeSegments` for `allSegments` in the
 * `trimByCutting` call inside `trimRecording` — the removed-idle-segment
 * assertion below fails because the idle segment's bounds show up in the
 * spawned ffmpeg args.
 */

import { EventEmitter } from 'events'
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ActivitySegment } from '../../src/main/runtime/video-activity-tracker'

const spawnedArgs: string[][] = []

vi.mock('child_process', () => ({
  spawn: vi.fn((_cmd: string, args: string[]) => {
    spawnedArgs.push(args)
    const proc = new EventEmitter() as EventEmitter & { stdout: EventEmitter; stderr: EventEmitter }
    proc.stdout = new EventEmitter()
    proc.stderr = new EventEmitter()
    queueMicrotask(() => proc.emit('close', 0))
    return proc
  }),
}))

const { trimRecording } = await import('../../src/main/runtime/video-trimmer')

describe('trimRecording (cut mode)', () => {
  let workDir: string
  let inputPath: string

  beforeEach(() => {
    spawnedArgs.length = 0
    workDir = mkdtempSync(join(tmpdir(), 'video-trimmer-test-'))
    inputPath = join(workDir, 'recording.webm')
    writeFileSync(inputPath, '')
  })

  afterEach(() => {
    rmSync(workDir, { recursive: true, force: true })
  })

  it('cuts at each active segment\'s own wall-clock bounds and drops the idle one', async () => {
    const segments: ActivitySegment[] = [
      { type: 'active', startTime: 0, endTime: 5, duration: 5 },
      { type: 'idle', startTime: 5, endTime: 12, duration: 7 },
      { type: 'active', startTime: 12, endTime: 20, duration: 8 },
    ]
    const outputPath = join(workDir, 'recording-trimmed.webm')

    const result = await trimRecording({ inputPath, segments, outputPath })

    // Two extraction calls (one per active segment) plus one concat call.
    expect(spawnedArgs).toHaveLength(3)
    expect(spawnedArgs[0]).toEqual(expect.arrayContaining(['-ss', '0', '-to', '5']))
    expect(spawnedArgs[1]).toEqual(expect.arrayContaining(['-ss', '12', '-to', '20']))
    // The idle segment's bounds never reach ffmpeg.
    for (const args of spawnedArgs) {
      expect(args).not.toEqual(expect.arrayContaining(['-ss', '5', '-to', '12']))
    }

    expect(result.originalDuration).toBe(20)
    expect(result.trimmedDuration).toBe(13)
    expect(result.segmentsRemoved).toBe(1)
    expect(result.segmentsKept).toBe(2)
  })

  it('keeps a short idle gap under minIdleMs instead of cutting it', async () => {
    const segments: ActivitySegment[] = [
      { type: 'active', startTime: 0, endTime: 5, duration: 5 },
      { type: 'idle', startTime: 5, endTime: 6, duration: 1 },
      { type: 'active', startTime: 6, endTime: 10, duration: 4 },
    ]
    const outputPath = join(workDir, 'recording-trimmed.webm')

    const result = await trimRecording({ inputPath, segments, outputPath, minIdleMs: 3000 })

    // Nothing crossed the threshold, so the whole file is kept verbatim
    // (a plain file copy, no ffmpeg spawn at all).
    expect(spawnedArgs).toHaveLength(0)
    expect(result.segmentsRemoved).toBe(0)
    expect(result.trimmedDuration).toBe(result.originalDuration)
    expect(existsSync(outputPath)).toBe(true)
  })
})
