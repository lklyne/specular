import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { CanvasItemSurface } from '../../../src/renderer/canvas-bg/CanvasItemSurface'

const root = createRoot(document.getElementById('root')!)
// A 1000px page restored at 20% zoom, with no animated content to mask a lost frame.
window.renderSurface = (visible = true, x = 0) => {
  root.render(
    <StrictMode>
      <CanvasItemSurface api={window.electronAPI} isDark={false} draws={visible ? [{
        pageId: 'static', chrome: false,
        item: { id: 'static', screenX: x, screenY: 0, screenWidth: 200, screenHeight: 150, width: 1000 },
      }] : []} />
    </StrictMode>,
  )
}
