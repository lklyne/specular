import type { SceneEntity } from './types'

/**
 * Image entities render bare — no card background/shadow and no filename
 * chrome. Centralised so the body card and chrome overlay agree on what
 * counts as a bare image. When a second bare renderer appears, promote this
 * to a presentation flag on the renderer claim instead of a tag check.
 */
export function isBareImageEntity(entity: SceneEntity): boolean {
  return entity.rendererTag === 'image'
}
