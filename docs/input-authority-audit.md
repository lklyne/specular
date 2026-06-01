# Canvas Input Authority Audit

Status: implemented. In canvas mode, canvas gestures enter through
`src/renderer/above-view/useCanvasPointerRouter.ts`; closed gate states are
reserved for native page input, inline editing, and inspect/comment probing.

## Live Path Classification

| Path | Classification | Result |
| --- | --- | --- |
| `src/renderer/above-view/useCanvasPointerRouter.ts` | keep as router-owned | Owns selection, drag, resize, marquee, pan, edge drag, and click-to-focus. |
| `src/renderer/above-view/App.tsx` | keep as router/tool-owned | Keeps explicit aboveView handlers for pending placement and region-select tool mode; wheel and middle-pan use the narrow viewport hook. |
| `src/renderer/shared/hooks/useViewportWheelAndMiddlePan.ts` | keep as native viewport-only | Forwards wheel zoom/pan and middle-button pan; yields wheel to a hovered page/note for native scroll. Canvas-mode authority. |
| `src/renderer/canvas-bg/App.tsx` | convert to visual/native-only | Renders grid, entities, menus, outlines, and accepts file drops; no canvas left-button gestures. |
| `src/renderer/canvas-bg/CanvasSelectionLayers.tsx` | convert to visual-only | Selection outlines and handles render without starting drag or resize. |
| `src/renderer/canvas-bg/EdgeLayer.tsx` | convert to visual-only | Edge paths and anchor affordances render without selecting or starting edge drags. |
| `src/renderer/canvas-bg/SelectionResizeGrid.tsx` | convert to visual-only | Handles paint only; router resize handles are hit-tested from shared geometry. |
| `src/renderer/canvas-bg/SelectableEntityShell.tsx` | convert to visual/editing-only | Entity cards render and host inline editors; drag/resize/select hooks removed. |
| `src/preload/page-content.ts` | keep as native/probing-only | Keeps page wheel, hover, inspect/comment probing, and peek resize; page content no longer selects or drags canvas frames. |
| `src/main/ipc/register-canvas-drag-ipc.ts` | keep as router IPC | Keeps router-driven drag/edge/viewport channels; removed legacy selection drag IPC. |
| `src/main/ipc/register-page-chrome-ipc.ts` | keep as native/page IPC | Keeps page deselect, hover, scroll, dropdown, and peek resize; removed page select/group-drag/marquee/region-select channels. |

## Focus Exit Policy

Focused frames receive native input while `frameFocus` is set. Escape exits
focus. Clicking outside a focused frame relies on the focused page
`webContents.blur` transition to clear focus; after focus exits, the next
canvas gesture is handled by aboveView. Canvas retargeting does not fall back
to bgView or page-content handlers.
