import type { CanvasSceneFileEntity } from '../../../shared/types'
import {
  HTML_EXTENSIONS,
  IMAGE_EXTENSIONS,
  MARKDOWN_EXTENSIONS,
  VIDEO_EXTENSIONS,
} from '../entityConstants'
import { ComponentPlaceholderRenderer } from './ComponentPlaceholderRenderer'
import { FileFallbackRenderer } from './FileFallbackRenderer'
import { HtmlInlineRenderer } from './HtmlInlineRenderer'
import { ImageInlineRenderer } from './ImageInlineRenderer'
import { MarkdownInlineRenderer } from './MarkdownInlineRenderer'
import { VideoInlineRenderer } from './VideoInlineRenderer'

/**
 * Pick the inline renderer for a file entity. The registry's rendererTag
 * (broadcast in scene data) is the canonical source; the extension regex
 * is a defensive backstop for entities that haven't been re-broadcast
 * since boot.
 */
function resolveTag(entity: CanvasSceneFileEntity): CanvasSceneFileEntity['rendererTag'] {
  if (entity.rendererTag) return entity.rendererTag
  if (IMAGE_EXTENSIONS.test(entity.file)) return 'image'
  if (VIDEO_EXTENSIONS.test(entity.file)) return 'video'
  if (MARKDOWN_EXTENSIONS.test(entity.file)) return 'markdown'
  if (HTML_EXTENSIONS.test(entity.file)) return 'html'
  return undefined
}

export function RendererSwitch({
  entity,
  canEdit,
  isDark,
  isInteractive,
  onTextEditingChange,
}: {
  entity: CanvasSceneFileEntity
  canEdit: boolean
  isDark: boolean
  /** The entered interactive file (HTML iframe): its content owns the pointer. */
  isInteractive: boolean
  onTextEditingChange: (active: boolean) => void
}) {
  const tag = resolveTag(entity)
  switch (tag) {
    case 'image':
      return <ImageInlineRenderer entity={entity} />
    case 'video':
      return <VideoInlineRenderer entity={entity} canEdit={canEdit} />
    case 'markdown':
      return (
        <MarkdownInlineRenderer
          entity={entity}
          canEdit={canEdit}
          isDark={isDark}
          onTextEditingChange={onTextEditingChange}
        />
      )
    case 'component':
      return <ComponentPlaceholderRenderer entity={entity} isDark={isDark} />
    case 'html':
      return <HtmlInlineRenderer entity={entity} isInteractive={isInteractive} />
    default:
      return <FileFallbackRenderer entity={entity} isDark={isDark} />
  }
}

