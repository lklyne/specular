import { MERMAID_EXTENSIONS } from '../../../shared/file-extensions'
import type { InlineRendererClaim } from '../registry'

/** `.mmd` / `.mermaid` files render as a diagram. Display-only: the file is
 *  the source, edited in an editor or by an agent, and the canvas re-renders
 *  when it changes on disk. */
export const mermaidRenderPlugin: InlineRendererClaim = {
  id: 'specular.mermaid',
  kind: 'inline',
  rendererTag: 'mermaid',
  editable: false,
  claims: (entity) => MERMAID_EXTENSIONS.test(entity.file),
}
