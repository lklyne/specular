import { execFile } from 'child_process'
import { mkdir, writeFile } from 'fs/promises'
import { app } from 'electron'
import path from 'path'
import { promisify } from 'util'
import { win } from './runtime/view-refs'

const execFileAsync = promisify(execFile)
const CAPTURE_TAIL_MS = 250

export interface WindowFrameSample {
  fileName: string
  startedAtMs: number
  durationMs: number
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Captures the complete native Specular window, including child
 * WebContentsViews. Electron renderer capture APIs cannot see that composed
 * result, and macOS window-video capture produces black WebContentsView frames.
 *
 * `screencapture` is intentionally run sequentially: it is only fast enough for
 * a low-frame-rate diagnostic sequence, but overlapping processes would distort
 * the performance trace much more severely.
 */
export async function captureWindowFramesWhile<T>(
  run: () => Promise<T>,
): Promise<{
  result: T
  frameDirectory: string
  manifestPath: string
  samples: WindowFrameSample[]
}> {
  if (process.platform !== 'darwin') {
    throw new Error('Full-window performance frame capture currently requires macOS')
  }
  if (!win || win.isDestroyed()) throw new Error('Main window is not ready')

  const windowId = win.getMediaSourceId().split(':')[1]
  if (!windowId || !/^\d+$/.test(windowId)) {
    throw new Error('Could not resolve the native Specular window id')
  }

  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  const frameDirectory = path.join(
    app.getPath('logs'),
    `specular-pan-zoom-${stamp}-frames`,
  )
  await mkdir(frameDirectory, { recursive: true })

  const captureStartedAt = performance.now()
  const samples: WindowFrameSample[] = []
  let frameIndex = 0
  let stopping = false

  const captureFrame = async (): Promise<void> => {
    const fileName = `${String(frameIndex).padStart(4, '0')}.png`
    frameIndex += 1
    const startedAt = performance.now()
    await execFileAsync('screencapture', [
      '-r',
      '-l',
      windowId,
      '-o',
      '-x',
      path.join(frameDirectory, fileName),
    ])
    samples.push({
      fileName,
      startedAtMs: startedAt - captureStartedAt,
      durationMs: performance.now() - startedAt,
    })
  }

  await captureFrame()
  const captureLoop = (async () => {
    while (!stopping) await captureFrame()
  })()

  let result: T
  try {
    result = await run()
    await wait(CAPTURE_TAIL_MS)
  } finally {
    stopping = true
    await captureLoop
  }

  const manifestPath = path.join(frameDirectory, 'manifest.json')
  await writeFile(
    manifestPath,
    `${JSON.stringify({
      capturedAt: new Date().toISOString(),
      windowId,
      frameCount: samples.length,
      samples,
    }, null, 2)}\n`,
    'utf8',
  )

  return { result, frameDirectory, manifestPath, samples }
}
