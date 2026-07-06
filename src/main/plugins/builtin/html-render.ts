import { HTML_EXTENSIONS } from '../../../shared/file-extensions'
import type { InlineRendererClaim } from '../registry'

export const htmlRenderPlugin: InlineRendererClaim = {
  id: 'specular.html',
  kind: 'inline',
  rendererTag: 'html',
  editable: false,
  interactive: true,
  claims: (entity) => HTML_EXTENSIONS.test(entity.file),
}
