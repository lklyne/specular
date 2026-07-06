import { MARKDOWN_EXTENSIONS } from '../../../shared/file-extensions'
import type { InlineRendererClaim } from '../registry'

export const markdownRenderPlugin: InlineRendererClaim = {
  id: 'specular.markdown',
  kind: 'inline',
  rendererTag: 'markdown',
  editable: true,
  claims: (entity) => MARKDOWN_EXTENSIONS.test(entity.file),
}
