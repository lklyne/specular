import { randomUUID } from 'crypto'
import { existsSync, mkdirSync, writeFileSync } from 'fs'
import { join } from 'path'
import { spaceDir } from './space-dir'

function assetsDir(): string {
  const dir = join(spaceDir(), 'assets')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

export function saveImageBuffer(buffer: Buffer, ext = 'png'): string {
  const filename = `${randomUUID()}.${ext}`
  const filePath = join(assetsDir(), filename)
  writeFileSync(filePath, buffer)
  return filePath
}
