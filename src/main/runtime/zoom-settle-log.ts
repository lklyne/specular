/**
 * INSTRUMENTATION for the zoom-settle / pan-stall investigation (pages flash
 * when a pan overlaps the zoom snapshot settle). Strip this file and every
 * `settleLog` call site before merging.
 *
 * Lines go to stdout and to `<logs>/zoom-settle.log` so an agent can read the
 * timeline from a spawned instance without a terminal.
 */
import { appendFile } from 'fs'
import { join } from 'path'

let resolvedPath: string | null | undefined

function logFilePath(): string | null {
  if (resolvedPath !== undefined) return resolvedPath
  try {
    // Lazy require so unit/integration runs (electron stubbed or absent)
    // degrade to console-only logging.
    const { app } = require('electron') as typeof import('electron')
    resolvedPath = app?.getPath ? join(app.getPath('logs'), 'zoom-settle.log') : null
  } catch {
    resolvedPath = null
  }
  return resolvedPath
}

export function settleLog(message: string): void {
  const line = `${new Date().toISOString()} +${performance.now().toFixed(1)} ${message}`
  console.log(`[zoom-settle] ${message}`)
  const file = logFilePath()
  if (file) appendFile(file, `${line}\n`, () => {})
}
