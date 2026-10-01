import { spawn, type ChildProcess } from 'child_process'
import { screen as electronScreen } from 'electron'
import { writeFileSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { randomUUID } from 'crypto'
import type { Page } from './runtime-entities'
import { win } from './window-shell'
import { VideoActivityTracker, type ActivitySegment } from './video-activity-tracker'
import { captureFrameComposited } from './frame-compositor'
import { getZoom, pan } from './runtime-context'
import { focusCanvasBounds, requestLayout, setPan, setZoom } from './viewport-control'
import { boundScreenBoundsForPage, pageBodyCanvasBounds } from './runtime-geometry'
import { holdPagesAwake } from './page-idle-throttle'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface RecordingOptions {
  page: Page
  outputPath?: string
  fps?: number
  quality?: 'high' | 'medium' | 'compact'
}

export interface RecordingState {
  status: 'idle' | 'recording'
  recordingId: string | null
  outputPath: string | null
  startedAt: number | null
  frameCount: number
  /** Frames written that repeat the last captured frame to hold the declared
   *  fps against wall-clock time, rather than fresh captures. High under
   *  slow capture; the video is still correct-length either way. */
  duplicatedFrames: number
  droppedFrames: number
  /** Wall-clock ms excluded from the timeline by a stall too long to
   *  reproduce frame-for-frame — see `computeFrameWrites`. Zero outside of
   *  a genuine stall (a blocked process, a system sleep). */
  stalledMs: number
  elapsed: number
}

interface QualityPreset {
  fps: number
  crf: number
}

// Presets differ in quality, not frame rate. A composited capture sustains
// well under 30 fresh frames a second, and ffmpeg can't convert and encode 60
// full-size frames a second alongside the running app — it falls behind and
// the recorder has to drop time to stay bounded.
const QUALITY_PRESETS: Record<string, QualityPreset> = {
  high: { fps: 30, crf: 20 },
  medium: { fps: 30, crf: 30 },
  compact: { fps: 30, crf: 40 },
}

/** Delay before retrying a failed composite capture, so a persistent failure
 *  (destroyed page, zero-size race) polls instead of spinning the event loop. */
const CAPTURE_RETRY_DELAY_MS = 100

// ---------------------------------------------------------------------------
// ffmpeg argument list
//
// Kept as one pure builder so the recorder and anything that needs to
// reproduce its exact pipe (a scratch script, a test) can't drift from it.
// ---------------------------------------------------------------------------

export interface FfmpegRecordArgsOptions {
  width: number
  height: number
  fps: number
  crf: number
  outputPath: string
}

/**
 * Raw BGRA frames arrive on stdin at a constant declared frame rate; the
 * caller is responsible for writing exactly `fps` frames per second of
 * wall-clock recording time (see `computeFrameWrites` below) so this stays a
 * plain constant-frame-rate input rather than needing timestamp-based (vfr)
 * flags, which would make output timing depend on stdin pipe scheduling.
 */
export function buildFfmpegRecordArgs(opts: FfmpegRecordArgsOptions): string[] {
  return [
    '-y',
    '-f', 'rawvideo',
    '-pixel_format', 'bgra',
    '-video_size', `${opts.width}x${opts.height}`,
    '-framerate', String(opts.fps),
    '-i', 'pipe:0',
    '-c:v', 'libvpx-vp9',
    '-crf', String(opts.crf),
    '-b:v', '0',
    '-deadline', 'realtime',
    '-cpu-used', '8',
    '-row-mt', '1',
    opts.outputPath,
  ]
}

// ---------------------------------------------------------------------------
// Frame pacing
//
// A composited capture (three capturePage calls plus a CPU alpha blend) can
// take far longer than one frame interval at the declared fps, so captures
// land at their own, slower, irregular cadence. Piping one written frame per
// capture would encode real wall-clock time as if every capture were exactly
// one frame apart — the recording plays back faster than it was made.
// Instead the capture loop and the write loop run independently: captures
// happen as fast as they can, and on every write tick this decides how many
// copies of the latest captured frame to emit so that
// `framesWritten / fps` tracks wall-clock elapsed time, duplicating frames
// under slow capture and writing nothing when capture is outpacing playback.
// ---------------------------------------------------------------------------

export interface FramePacingState {
  framesWritten: number
  /** Wall-clock time (ms) folded out of the timeline by a stall too long to
   *  reproduce frame-for-frame — see `computeFrameWrites`. */
  droppedMs: number
}

export const INITIAL_FRAME_PACING_STATE: FramePacingState = { framesWritten: 0, droppedMs: 0 }

export interface FrameWritePlan {
  framesToWrite: number
  state: FramePacingState
}

/**
 * `elapsedMs` is wall-clock time since recording start. Under steady or
 * merely slow capture, the write loop's own tick interval never lets a gap
 * bigger than `maxCatchUpFrames` build up between calls, so the cap is never
 * hit — duplication alone keeps pace. It only fires after a genuine stall
 * (the process itself was blocked, or the machine slept), where a single
 * call can see many seconds of elapsed time appear at once. Reproducing that
 * frame-for-frame would mean writing an unbounded burst of duplicate frames
 * (hours of 16MB buffers for a laptop-sleep-length stall); silently capping
 * the burst without remembering the shortfall would instead quietly
 * re-introduce a compressed timeline for everything recorded afterward. So
 * the excess beyond the cap is dropped from the timeline once, and
 * `droppedMs` carries the amount forward so later calls keep computing
 * against wall-clock time honestly (the video freezes on the last frame for
 * `maxCatchUpFrames` worth of time across the stall, then resumes exact
 * tracking) rather than either extreme.
 */
export function computeFrameWrites(
  state: FramePacingState,
  elapsedMs: number,
  fps: number,
  maxCatchUpFrames: number,
): FrameWritePlan {
  const effectiveElapsedMs = elapsedMs - state.droppedMs
  const targetFrameCount = Math.max(0, Math.floor((effectiveElapsedMs / 1000) * fps))
  const needed = targetFrameCount - state.framesWritten
  if (needed <= 0) {
    return { framesToWrite: 0, state }
  }
  if (needed <= maxCatchUpFrames) {
    return {
      framesToWrite: needed,
      state: { framesWritten: state.framesWritten + needed, droppedMs: state.droppedMs },
    }
  }
  const droppedFrames = needed - maxCatchUpFrames
  return {
    framesToWrite: maxCatchUpFrames,
    state: {
      framesWritten: state.framesWritten + maxCatchUpFrames,
      droppedMs: state.droppedMs + (droppedFrames / fps) * 1000,
    },
  }
}

// ---------------------------------------------------------------------------
// VideoRecorder
// ---------------------------------------------------------------------------

let activeRecorder: VideoRecorderInstance | null = null

class VideoRecorderInstance {
  private ffmpeg: ChildProcess | null = null
  private writeTimer: NodeJS.Timeout | null = null
  /** Most recently completed composite; the write loop duplicates it to hold
   *  pace when captures land slower than the declared fps. */
  private latestFrame: Buffer | null = null
  /** Recording composites live frames on a timer, with no traffic of its own
   *  to prove the page is in use — hold it awake for the duration. */
  private releaseAwakeHold: (() => void) | null = null

  readonly recordingId: string
  readonly outputPath: string
  readonly fps: number
  readonly crf: number
  readonly activityTracker: VideoActivityTracker
  private readonly page: Page

  private startedAt = 0
  private frameCount = 0
  private capturedFrameCount = 0
  private droppedFrames = 0
  private pacing: FramePacingState = INITIAL_FRAME_PACING_STATE
  private captureWidth = 0
  private captureHeight = 0
  /** One second of frames: enough to ride out an encoder hiccup, small enough
   *  that a stuck encoder can't exhaust memory. Set once the size is known. */
  private maxQueuedBytes = Number.POSITIVE_INFINITY
  private dpr = 1
  private stopped = false
  private savedCamera: { zoom: number; panX: number; panY: number } | null = null

  constructor(options: RecordingOptions) {
    this.recordingId = randomUUID()
    this.page = options.page
    const preset = QUALITY_PRESETS[options.quality ?? 'medium']
    this.fps = options.fps ?? preset.fps
    this.crf = preset.crf
    this.outputPath =
      options.outputPath ?? join(tmpdir(), `web-canvas-recording-${this.recordingId}.webm`)
    this.activityTracker = new VideoActivityTracker()
  }

  async start(): Promise<void> {
    if (this.page.host.webContents.isDestroyed()) {
      throw new Error('Target page webContents is destroyed')
    }
    this.releaseAwakeHold = holdPagesAwake()
    const w = win
    if (!w || w.isDestroyed()) {
      throw new Error('Window not available')
    }

    // Each frame composites the annotation and cursor overlays, which are
    // drawn at canvas zoom, onto the page. They only register with the page at
    // 100%, so the camera is pinned to zoom 1 (and onto the page) for the
    // duration of the recording and restored on stop.
    this.savedCamera = { zoom: getZoom(), panX: pan.x, panY: pan.y }
    try {
      if (getZoom() !== 1) setZoom(1)
      focusCanvasBounds(pageBodyCanvasBounds(this.page))
      requestLayout()
      // Give the page a beat to repaint at the settled camera so the first
      // captured frames aren't mid-transition.
      await new Promise((r) => setTimeout(r, 250))

      const display = electronScreen.getDisplayMatching(w.getBounds())
      this.dpr = display.scaleFactor
      // The composited frame arrives at the page's projected on-screen size;
      // the camera is pinned at zoom 1 above, so this is also its CSS size.
      const pageRect = boundScreenBoundsForPage(this.page).page
      this.captureWidth = Math.round(pageRect.width * this.dpr)
      this.captureHeight = Math.round(pageRect.height * this.dpr)
      this.maxQueuedBytes = this.captureWidth * this.captureHeight * 4 * this.fps

      if (this.captureWidth === 0 || this.captureHeight === 0) {
        throw new Error('Canvas view has zero dimensions')
      }

      // Spawn ffmpeg to accept raw BGRA frames on stdin.
      // Use VP9 with realtime deadline for fast encoding at high resolution.
      this.ffmpeg = spawn(
        'ffmpeg',
        buildFfmpegRecordArgs({
          width: this.captureWidth,
          height: this.captureHeight,
          fps: this.fps,
          crf: this.crf,
          outputPath: this.outputPath,
        }),
        { stdio: ['pipe', 'pipe', 'pipe'] },
      )

      this.ffmpeg.on('error', (err) => {
        console.error('[video-recorder] ffmpeg error:', err.message)
      })

      this.ffmpeg.stderr?.on('data', (chunk: Buffer) => {
        // Log ffmpeg progress/errors but don't spam.
        const msg = chunk.toString().trim()
        if (msg.includes('Error') || msg.includes('error')) {
          console.error('[video-recorder] ffmpeg:', msg)
        }
      })

      this.startedAt = Date.now()
      this.activityTracker.start()

      // Capture and write run on independent schedules: capture as fast as
      // the composite pipeline sustains, and pace writes off the wall clock
      // (see computeFrameWrites) so the declared fps encodes real elapsed
      // time regardless of how often a fresh capture actually lands.
      void this.runCaptureLoop()
      const intervalMs = Math.round(1000 / this.fps)
      this.writeTimer = setInterval(() => this.paceWrites(), intervalMs)
    } catch (error) {
      this.releaseAwakeHold?.()
      this.restoreCamera()
      throw error
    }
  }

  private async runCaptureLoop(): Promise<void> {
    while (!this.stopped) {
      if (!this.ffmpeg?.stdin || this.ffmpeg.stdin.destroyed) {
        this.droppedFrames++
        return
      }

      try {
        const result = await captureFrameComposited(this.page, { dpr: this.dpr })
        if (result && result.width === this.captureWidth && result.height === this.captureHeight) {
          this.latestFrame = result.bitmap
          this.capturedFrameCount++
          continue
        }
        this.droppedFrames++
      } catch (error) {
        console.error('[video-recorder] capture frame error:', error)
        this.droppedFrames++
      }

      await new Promise((r) => setTimeout(r, CAPTURE_RETRY_DELAY_MS))
    }
  }

  private paceWrites(): void {
    if (this.stopped || !this.latestFrame) return
    if (!this.ffmpeg?.stdin || this.ffmpeg.stdin.destroyed) return

    // An encoder that falls behind would otherwise grow stdin's queue without
    // bound, each queued frame pinning a full-size bitmap. Skipping the tick
    // turns the backlog into an elapsed-time jump, which the catch-up cap
    // below already accounts for as a stall.
    if (this.ffmpeg.stdin.writableLength > this.maxQueuedBytes) return

    const elapsedMs = Date.now() - this.startedAt
    // A one-second catch-up cap: ordinary slow capture never builds a
    // backlog bigger than a tick's worth (this timer duplicates the latest
    // frame regardless of capture cadence), so hitting the cap means a real
    // stall, not the compositor being slow.
    const plan = computeFrameWrites(this.pacing, elapsedMs, this.fps, this.fps)
    this.pacing = plan.state

    // Node.js buffers internally even when write() returns false
    // (backpressure hint). At ~16MB per frame every write exceeds the
    // default highWaterMark, but the data is still queued and ffmpeg
    // consumes it at its encoding pace; writing the same buffer object
    // `framesToWrite` times costs no extra allocation.
    for (let i = 0; i < plan.framesToWrite; i++) {
      this.ffmpeg.stdin.write(this.latestFrame)
    }
    this.frameCount += plan.framesToWrite
  }

  async stop(): Promise<{
    outputPath: string
    duration: number
    frameCount: number
    duplicatedFrames: number
    droppedFrames: number
    stalledMs: number
    segments: ActivitySegment[]
  }> {
    this.stopped = true
    this.releaseAwakeHold?.()

    if (this.writeTimer) {
      clearInterval(this.writeTimer)
      this.writeTimer = null
    }

    this.activityTracker.stop()
    const segments = this.activityTracker.getSegments()

    // Restore pre-recording camera. Done before ffmpeg flush so the user's
    // canvas snaps back immediately; the video finishes encoding in the
    // background.
    this.restoreCamera()

    // Write segments metadata alongside the video.
    const segmentsPath = this.outputPath.replace(/\.webm$/, '-segments.json')
    writeFileSync(segmentsPath, JSON.stringify({ segments }, null, 2))

    // Close ffmpeg stdin and wait for it to finish encoding.
    await new Promise<void>((resolve) => {
      if (!this.ffmpeg) {
        resolve()
        return
      }
      if (this.ffmpeg.exitCode !== null) {
        resolve()
        return
      }
      this.ffmpeg.on('close', () => resolve())
      this.ffmpeg.stdin?.end()
    })

    const duration = (Date.now() - this.startedAt) / 1000

    return {
      outputPath: this.outputPath,
      duration,
      frameCount: this.frameCount,
      duplicatedFrames: this.duplicatedFrameCount(),
      droppedFrames: this.droppedFrames,
      stalledMs: this.pacing.droppedMs,
      segments,
    }
  }

  private duplicatedFrameCount(): number {
    return Math.max(0, this.frameCount - this.capturedFrameCount)
  }

  private restoreCamera(): void {
    if (!this.savedCamera) return
    const saved = this.savedCamera
    this.savedCamera = null
    if (getZoom() !== saved.zoom) setZoom(saved.zoom)
    setPan(saved.panX, saved.panY)
    requestLayout()
  }

  getState(): RecordingState {
    return {
      status: 'recording',
      recordingId: this.recordingId,
      outputPath: this.outputPath,
      startedAt: this.startedAt,
      frameCount: this.frameCount,
      duplicatedFrames: this.duplicatedFrameCount(),
      droppedFrames: this.droppedFrames,
      stalledMs: this.pacing.droppedMs,
      elapsed: (Date.now() - this.startedAt) / 1000,
    }
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function startRecording(options: RecordingOptions): Promise<{
  recordingId: string
  outputPath: string
}> {
  if (activeRecorder) {
    throw new Error('Recording already in progress')
  }

  // Verify ffmpeg is available.
  await verifyFfmpeg()

  activeRecorder = new VideoRecorderInstance(options)
  await activeRecorder.start()

  return {
    recordingId: activeRecorder.recordingId,
    outputPath: activeRecorder.outputPath,
  }
}

export async function stopRecording(): Promise<{
  outputPath: string
  segmentsPath: string
  duration: number
  frameCount: number
  duplicatedFrames: number
  droppedFrames: number
  stalledMs: number
  segments: ActivitySegment[]
}> {
  if (!activeRecorder) {
    throw new Error('No recording in progress')
  }

  const result = await activeRecorder.stop()
  activeRecorder = null

  return {
    ...result,
    segmentsPath: result.outputPath.replace(/\.webm$/, '-segments.json'),
  }
}

export function getRecordingState(): RecordingState {
  if (!activeRecorder) {
    return {
      status: 'idle',
      recordingId: null,
      outputPath: null,
      startedAt: null,
      frameCount: 0,
      duplicatedFrames: 0,
      droppedFrames: 0,
      stalledMs: 0,
      elapsed: 0,
    }
  }
  return activeRecorder.getState()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let ffmpegVerified = false

async function verifyFfmpeg(): Promise<void> {
  if (ffmpegVerified) return
  const { execFile } = await import('child_process')
  const { promisify } = await import('util')
  const exec = promisify(execFile)
  try {
    await exec('ffmpeg', ['-version'])
    ffmpegVerified = true
  } catch {
    throw new Error(
      'ffmpeg not found on PATH. Install it with: brew install ffmpeg (macOS) or apt install ffmpeg (Linux)',
    )
  }
}
