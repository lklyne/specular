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
import { pageBodyCanvasBounds } from './runtime-geometry'
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
  droppedFrames: number
  elapsed: number
}

interface QualityPreset {
  fps: number
  crf: number
}

const QUALITY_PRESETS: Record<string, QualityPreset> = {
  high: { fps: 60, crf: 20 },
  medium: { fps: 30, crf: 30 },
  compact: { fps: 30, crf: 40 },
}

// ---------------------------------------------------------------------------
// VideoRecorder
// ---------------------------------------------------------------------------

let activeRecorder: VideoRecorderInstance | null = null

class VideoRecorderInstance {
  private ffmpeg: ChildProcess | null = null
  private captureTimer: NodeJS.Timeout | null = null
  private capturing = false
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
  private droppedFrames = 0
  private captureWidth = 0
  private captureHeight = 0
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
    if (this.page.pageView.webContents.isDestroyed()) {
      throw new Error('Target page webContents is destroyed')
    }
    this.releaseAwakeHold = holdPagesAwake()
    const w = win
    if (!w || w.isDestroyed()) {
      throw new Error('Window not available')
    }

    // Canvas zoom is wired into Chromium's device emulation `scale` in
    // computeApplyEmulation — the page is actually rendered into only the
    // scaled sub-region of its emulated viewport. To capture at native size,
    // the page must be rendered at native size, which means forcing canvas
    // zoom to 1 for the duration of the recording.
    this.savedCamera = { zoom: getZoom(), panX: pan.x, panY: pan.y }
    try {
      if (getZoom() !== 1) setZoom(1)
      focusCanvasBounds(pageBodyCanvasBounds(this.page))
      requestLayout()
      // Chromium re-rasters on the next frame after enableDeviceEmulation.
      // Give it room so the first captured frames aren't mid-transition.
      await new Promise((r) => setTimeout(r, 250))

      const display = electronScreen.getDisplayMatching(w.getBounds())
      this.dpr = display.scaleFactor
      const pageBounds = this.page.pageView.getBounds()
      this.captureWidth = Math.round(pageBounds.width * this.dpr)
      this.captureHeight = Math.round(pageBounds.height * this.dpr)

      if (this.captureWidth === 0 || this.captureHeight === 0) {
        throw new Error('Canvas view has zero dimensions')
      }

      // Spawn ffmpeg to accept raw BGRA frames on stdin.
      // Use VP9 with realtime deadline for fast encoding at high resolution.
      this.ffmpeg = spawn('ffmpeg', [
        '-y',
        '-f', 'rawvideo',
        '-pixel_format', 'bgra',
        '-video_size', `${this.captureWidth}x${this.captureHeight}`,
        '-framerate', String(this.fps),
        '-i', 'pipe:0',
        '-c:v', 'libvpx-vp9',
        '-crf', String(this.crf),
        '-b:v', '0',
        '-deadline', 'realtime',
        '-cpu-used', '8',
        '-row-mt', '1',
        this.outputPath,
      ], {
        stdio: ['pipe', 'pipe', 'pipe'],
      })

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

      // Start the capture loop.
      const intervalMs = Math.round(1000 / this.fps)
      this.captureTimer = setInterval(() => this.captureFrame(), intervalMs)
    } catch (error) {
      this.releaseAwakeHold?.()
      this.restoreCamera()
      throw error
    }
  }

  private async captureFrame(): Promise<void> {
    // Prevent overlapping captures.
    if (this.capturing || this.stopped) return
    this.capturing = true

    try {
      if (!this.ffmpeg || !this.ffmpeg.stdin || this.ffmpeg.stdin.destroyed) {
        this.droppedFrames++
        return
      }

      const result = await captureFrameComposited(this.page, { dpr: this.dpr })
      if (!result) {
        this.droppedFrames++
        return
      }

      if (result.width !== this.captureWidth || result.height !== this.captureHeight) {
        this.droppedFrames++
        return
      }

      // Write the raw BGRA frame to ffmpeg. Node.js will buffer internally
      // even when write() returns false (backpressure hint). At ~23MB per frame,
      // every write exceeds the default 16KB highWaterMark, but the data is
      // still queued and ffmpeg consumes it at its encoding pace.
      this.ffmpeg.stdin.write(result.bitmap)
      this.frameCount++
    } catch (error) {
      console.error('[video-recorder] capture frame error:', error)
      this.droppedFrames++
    } finally {
      this.capturing = false
    }
  }

  async stop(): Promise<{
    outputPath: string
    duration: number
    frameCount: number
    droppedFrames: number
    segments: ActivitySegment[]
  }> {
    this.stopped = true
    this.releaseAwakeHold?.()

    if (this.captureTimer) {
      clearInterval(this.captureTimer)
      this.captureTimer = null
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
      droppedFrames: this.droppedFrames,
      segments,
    }
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
      droppedFrames: this.droppedFrames,
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
  droppedFrames: number
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
      droppedFrames: 0,
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
