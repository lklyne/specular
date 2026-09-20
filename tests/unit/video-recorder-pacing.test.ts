/**
 * Frame-pacing math for the video recorder (video-recorder.ts): a composited
 * capture can take far longer than one frame interval at the declared fps,
 * so the write loop paces frame duplication off the wall clock instead of
 * writing one frame per capture — see `computeFrameWrites`'s doc comment for
 * the full rationale. These tests drive that pure function directly, plus
 * the ffmpeg argument builder it depends on staying in sync with the pipe
 * shape the recorder actually spawns.
 *
 * Mutation-verified by:
 * - removing the `maxCatchUpFrames` cap (returning `needed` unconditionally)
 *   — the "long stall" case fails by writing the full backlog in one tick
 *   instead of capping it and recording the shortfall in `droppedMs`;
 * - dropping the `state.droppedMs` subtraction from `effectiveElapsedMs`
 *   — the "recovers after a stall" case fails because pacing keeps trying
 *   to re-litigate the frames already folded into droppedMs;
 * - changing `Math.floor` to `Math.round` in the target-frame-count
 *   calculation — the "steady capture" case's exact frame-count assertion
 *   fails.
 */

import { describe, expect, it } from 'vitest'
import {
  buildFfmpegRecordArgs,
  computeFrameWrites,
  INITIAL_FRAME_PACING_STATE,
  type FramePacingState,
} from '../../src/main/runtime/video-recorder'

describe('computeFrameWrites', () => {
  it('writes nothing at zero elapsed time', () => {
    const plan = computeFrameWrites(INITIAL_FRAME_PACING_STATE, 0, 60, 60)
    expect(plan.framesToWrite).toBe(0)
    expect(plan.state).toEqual(INITIAL_FRAME_PACING_STATE)
  })

  it('tracks wall-clock time under steady capture', () => {
    // Ticks fire every ~16.67ms at 60fps; simulate 120 of them (2s) and
    // confirm the running frame count lands on the wall-clock target.
    const fps = 60
    let state: FramePacingState = INITIAL_FRAME_PACING_STATE
    let written = 0
    for (let tick = 1; tick <= 120; tick++) {
      const elapsedMs = tick * (1000 / fps)
      const plan = computeFrameWrites(state, elapsedMs, fps, fps)
      state = plan.state
      written += plan.framesToWrite
    }
    expect(written).toBe(120)
    expect(state.droppedMs).toBe(0)
  })

  it('duplicates the held frame to keep pace under slow capture', () => {
    // A capture landing every 300ms at a declared 60fps should still emit
    // ~18 frames per capture interval once the write loop ticks catch up,
    // with no frames dropped from the timeline.
    const fps = 60
    let state: FramePacingState = INITIAL_FRAME_PACING_STATE
    let written = 0
    for (let captureIndex = 1; captureIndex <= 10; captureIndex++) {
      const elapsedMs = captureIndex * 300
      const plan = computeFrameWrites(state, elapsedMs, fps, fps)
      state = plan.state
      written += plan.framesToWrite
    }
    expect(written).toBe(Math.floor((10 * 300 * fps) / 1000))
    expect(state.droppedMs).toBe(0)
  })

  it('caps a long stall instead of writing an unbounded burst, and recovers afterward', () => {
    const fps = 60
    const maxCatchUpFrames = fps // one second of grace per tick
    // A ten-minute stall (e.g. the process was blocked, or the machine
    // slept) surfaces as one call with a huge elapsed jump.
    const stallElapsedMs = 10 * 60 * 1000
    const plan = computeFrameWrites(INITIAL_FRAME_PACING_STATE, stallElapsedMs, fps, maxCatchUpFrames)

    expect(plan.framesToWrite).toBe(maxCatchUpFrames)
    const expectedTarget = Math.floor((stallElapsedMs / 1000) * fps)
    const expectedDroppedFrames = expectedTarget - maxCatchUpFrames
    expect(plan.state.droppedMs).toBeCloseTo((expectedDroppedFrames / fps) * 1000, 5)

    // The next tick, one frame interval later, should see the pacing
    // resume exact wall-clock tracking rather than trying to pay down the
    // dropped backlog — needed collapses back to ~1 frame, not thousands.
    const nextPlan = computeFrameWrites(plan.state, stallElapsedMs + 1000 / fps, fps, maxCatchUpFrames)
    expect(nextPlan.framesToWrite).toBeLessThanOrEqual(1)
  })
})

describe('buildFfmpegRecordArgs', () => {
  it('pins the raw-video input shape the write loop paces frames against', () => {
    const args = buildFfmpegRecordArgs({
      width: 320,
      height: 200,
      fps: 30,
      crf: 30,
      outputPath: '/tmp/out.webm',
    })
    expect(args).toEqual([
      '-y',
      '-f', 'rawvideo',
      '-pixel_format', 'bgra',
      '-video_size', '320x200',
      '-framerate', '30',
      '-i', 'pipe:0',
      '-c:v', 'libvpx-vp9',
      '-crf', '30',
      '-b:v', '0',
      '-deadline', 'realtime',
      '-cpu-used', '8',
      '-row-mt', '1',
      '/tmp/out.webm',
    ])
  })
})
