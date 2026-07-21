import { randomUUID } from 'crypto'
import { existsSync, mkdirSync, writeFileSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'
import { DEFAULT_WORKSPACE_ID } from './workspace-persistence'

export function assetsDir(): string {
  const dir = join(app.getPath('userData'), 'workspaces', DEFAULT_WORKSPACE_ID, 'assets')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

export function saveImageBuffer(buffer: Buffer, ext = 'png'): string {
  const filename = `${randomUUID()}.${ext}`
  const filePath = join(assetsDir(), filename)
  writeFileSync(filePath, buffer)
  return filePath
}
