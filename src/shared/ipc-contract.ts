import type {
  AgentPresenceCursor,
  AnnotationElementSelectionPayload,
  AnnotationLiveBboxUpdate,
  AppThemeMode,
  ConnectedRepo,
  DevtoolsPanelData,
  FixConfig,
  InteractionSyncCapturePayload,
  InteractionSyncEvent,
  LayoutUpdateData,
  LocatorResolveRequest,
  LocatorResolveResponse,
  LeftSidebarData,
  OnboardingProgressEvent,
  SelectionOverlayPayload,
  ThemeData,
  ToolbarSelectionData,
  ViewportNudge,
  WorkspaceBounds,
} from './types'
import type { BindingId } from './bindings'
import type { CanvasGuidesPayload } from './canvas-guides'
import type { PerfTraceState } from './electron-api/debug'
import type { PanZoomPerfTestState } from './pan-zoom-perf-test'

/**
 * The single source of truth for every IPC channel: its payload type and which
 * way it travels. Preload helpers (`on` / `send`) and main-side senders key off
 * this map, so a channel rename or payload drift becomes a compile error rather
 * than a runtime silence.
 *
 * Direction:
 *   'renderer→main'  renderer sends up (`ipcRenderer.send` / `ipcMain.on`)
 *   'main→renderer'  main broadcasts down (`webContents.send` / `on`)
 *   'invoke'         request/response (`ipcRenderer.invoke` / `ipcMain.handle`)
 *   'both'           the same channel travels both ways
 *
 * Payload types stay in `./types` (and sibling modules); this file only owns the
 * channel↔payload↔direction wiring. A channel's string literal lives here and
 * nowhere else in the codebase. Payloads that a preload `on` subscriber or a
 * bootstrap consumer reads are typed precisely; channels whose payload is only
 * validated at the main-side handler boundary are typed `unknown`.
 */
export interface IpcContract {
  'theme-changed': { dir: 'main→renderer'; payload: ThemeData }
  'layout-update': { dir: 'main→renderer'; payload: LayoutUpdateData }
  'aboveview-cursor-update': { dir: 'main→renderer'; payload: { type: string | null } }
  'agent-presence-changed': { dir: 'main→renderer'; payload: AgentPresenceCursor[] }
  'annotate-clear-hover': { dir: 'main→renderer'; payload: unknown }
  'annotate-element-selected': { dir: 'main→renderer'; payload: AnnotationElementSelectionPayload }
  'annotation-bbox-subscriptions': { dir: 'main→renderer'; payload: unknown }
  'annotation-bbox-update': { dir: 'renderer→main'; payload: unknown }
  'annotation-live-bbox': { dir: 'main→renderer'; payload: AnnotationLiveBboxUpdate }
  'annotation-open-thread': { dir: 'renderer→main'; payload: unknown }
  'annotation-thread-open': { dir: 'main→renderer'; payload: { annotationId: string } }
  'apply-linked-scroll': { dir: 'main→renderer'; payload: unknown }
  'apply-page-overrides': { dir: 'main→renderer'; payload: unknown }
  'binding-fire': { dir: 'main→renderer'; payload: BindingId }
  'canvas-back-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-bg-dropdown-close': { dir: 'renderer→main'; payload: unknown }
  'canvas-bg-dropdown-open': { dir: 'renderer→main'; payload: unknown }
  'canvas-cancel-entity-edit': { dir: 'renderer→main'; payload: unknown }
  'canvas-clear-annotate-hover': { dir: 'renderer→main'; payload: unknown }
  'canvas-comment-click-at': { dir: 'renderer→main'; payload: unknown }
  'canvas-commit-entity-edit': { dir: 'renderer→main'; payload: unknown }
  'canvas-commit-region-select': { dir: 'renderer→main'; payload: unknown }
  'canvas-copy-file-as-png': { dir: 'renderer→main'; payload: unknown }
  'canvas-copy-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-create-annotation': { dir: 'renderer→main'; payload: unknown }
  'canvas-create-drawing': { dir: 'renderer→main'; payload: unknown }
  'canvas-create-region-annotation': { dir: 'renderer→main'; payload: unknown }
  'canvas-create-tab': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-drawing-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-edge': { dir: 'renderer→main'; payload: unknown }
  'canvas-update-edge': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-file-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-shape': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-tab': { dir: 'renderer→main'; payload: unknown }
  'canvas-delete-text-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-arrange-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-copy-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-copy-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-entity-end': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-entity-start': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-group-end': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-group-start': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-page-end': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-page-start': { dir: 'renderer→main'; payload: unknown }
  'canvas-drag-preview': { dir: 'renderer→main'; payload: unknown }
  'canvas-drop-component-path': { dir: 'renderer→main'; payload: unknown }
  'canvas-drop-file-buffer': { dir: 'renderer→main'; payload: unknown }
  'canvas-duplicate-drawing-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-duplicate-file-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-duplicate-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-duplicate-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-duplicate-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-duplicate-shape': { dir: 'renderer→main'; payload: unknown }
  'canvas-duplicate-text-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-edge-drag-begin': { dir: 'renderer→main'; payload: unknown }
  'canvas-edge-drag-cancel': { dir: 'renderer→main'; payload: unknown }
  'canvas-edge-drag-commit': { dir: 'renderer→main'; payload: unknown }
  'canvas-edge-drag-target-change': { dir: 'renderer→main'; payload: unknown }
  'canvas-edge-edit-commit': { dir: 'renderer→main'; payload: unknown }
  'canvas-edge-edit-discard': { dir: 'renderer→main'; payload: unknown }
  'canvas-edit-component-prop': { dir: 'renderer→main'; payload: unknown }
  'canvas-edit-component-token': { dir: 'renderer→main'; payload: unknown }
  'canvas-enter-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-enter-page-interactive': { dir: 'renderer→main'; payload: unknown }
  'canvas-focus-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-forward-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-forward-pointer': { dir: 'renderer→main'; payload: unknown }
  'canvas-forward-wheel': { dir: 'renderer→main'; payload: unknown }
  'canvas-gap-resize-cancel': { dir: 'renderer→main'; payload: unknown }
  'canvas-gap-resize-commit': { dir: 'renderer→main'; payload: unknown }
  'canvas-gap-resize-move': { dir: 'renderer→main'; payload: unknown }
  'canvas-gap-resize-start': { dir: 'renderer→main'; payload: unknown }
  'canvas-group-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-guides': { dir: 'main→renderer'; payload: CanvasGuidesPayload }
  'canvas-hover-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-multi-resize-begin': { dir: 'renderer→main'; payload: unknown }
  'canvas-multi-resize-end': { dir: 'renderer→main'; payload: unknown }
  'canvas-navigate-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-open-devtools-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-pan': { dir: 'renderer→main'; payload: unknown }
  'canvas-paste-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-place-pending-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-reload-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-rename-drawing-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-rename-file-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-rename-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-rename-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-rename-tab': { dir: 'renderer→main'; payload: unknown }
  'canvas-rename-text-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-reorder-cancel': { dir: 'renderer→main'; payload: unknown }
  'canvas-reorder-commit': { dir: 'renderer→main'; payload: unknown }
  'canvas-reorder-move': { dir: 'renderer→main'; payload: unknown }
  'canvas-reorder-sidebar-item': { dir: 'renderer→main'; payload: unknown }
  'canvas-reorder-stack': { dir: 'renderer→main'; payload: unknown }
  'canvas-reorder-start': { dir: 'renderer→main'; payload: unknown }
  'canvas-reorder-tab': { dir: 'renderer→main'; payload: unknown }
  'canvas-request-entity-edit': { dir: 'renderer→main'; payload: unknown }
  'canvas-resize-begin': { dir: 'renderer→main'; payload: unknown }
  'canvas-resize-end': { dir: 'renderer→main'; payload: unknown }
  'canvas-resize-multi-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-restore-focus-camera': { dir: 'renderer→main'; payload: unknown }
  'canvas-reveal-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-reveal-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-reveal-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-select-edge': { dir: 'renderer→main'; payload: unknown }
  'canvas-select-entities': { dir: 'renderer→main'; payload: unknown }
  'canvas-select-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-select-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-select-in-rect': { dir: 'renderer→main'; payload: unknown }
  'canvas-select-page': { dir: 'renderer→main'; payload: unknown }
  'canvas-select-tab': { dir: 'renderer→main'; payload: unknown }
  'canvas-selection-overlay': { dir: 'both'; payload: SelectionOverlayPayload | null }
  'canvas-set-annotation-state': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-device-orientation': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-file-device-orientation': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-focus-annotations-visible': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-focus-presentation-mode': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-page-color-scheme': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-page-custom': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-page-preset': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-selection-preset': { dir: 'renderer→main'; payload: unknown }
  'canvas-set-text-editing': { dir: 'renderer→main'; payload: unknown }
  'canvas-show-file-in-finder': { dir: 'renderer→main'; payload: unknown }
  'canvas-show-page-context-menu': { dir: 'renderer→main'; payload: unknown }
  'canvas-toggle-annotate-mode': { dir: 'renderer→main'; payload: unknown }
  'canvas-toggle-device-shell': { dir: 'renderer→main'; payload: unknown }
  'canvas-toggle-draw-mode': { dir: 'renderer→main'; payload: unknown }
  'canvas-toggle-file-device-shell': { dir: 'renderer→main'; payload: unknown }
  'canvas-toggle-sync-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-unsync-page': { dir: 'renderer→main'; payload: string }
  'canvas-ungroup-group': { dir: 'renderer→main'; payload: unknown }
  'canvas-ungroup-selection': { dir: 'renderer→main'; payload: unknown }
  'canvas-update-entity': { dir: 'renderer→main'; payload: unknown }
  'canvas-update-page-bounds': { dir: 'renderer→main'; payload: unknown }
  'canvas-zoom': { dir: 'renderer→main'; payload: unknown }
  'capture-mode': { dir: 'main→renderer'; payload: boolean }
  'comment-canvas-point-committed': { dir: 'main→renderer'; payload: { canvasX: number; canvasY: number } }
  'comment-overlay-set-active': { dir: 'renderer→main'; payload: unknown }
  'comment-tool-bbox-subscriptions': { dir: 'renderer→main'; payload: unknown }
  'comment-tool-page-preview': { dir: 'main→renderer'; payload: unknown }
  'comment-tool-pointer-state': { dir: 'renderer→main'; payload: unknown }
  'component-tree-data': { dir: 'main→renderer'; payload: unknown }
  'cursor-spline-viz-changed': { dir: 'main→renderer'; payload: boolean }
  'debug-log': { dir: 'renderer→main'; payload: unknown }
  'debug:get-initial-data': { dir: 'invoke'; payload: unknown }
  'debug:perf-pan-zoom-get-state': { dir: 'invoke'; payload: unknown }
  'debug:perf-pan-zoom-run': { dir: 'invoke'; payload: unknown }
  'debug:perf-pan-zoom-state-changed': { dir: 'main→renderer'; payload: PanZoomPerfTestState }
  'debug:perf-pan-zoom-stop': { dir: 'invoke'; payload: unknown }
  'debug:perf-trace-get-state': { dir: 'invoke'; payload: unknown }
  'debug:perf-trace-get-summary': { dir: 'invoke'; payload: unknown }
  'debug:perf-trace-list': { dir: 'invoke'; payload: unknown }
  'debug:perf-trace-reveal': { dir: 'renderer→main'; payload: unknown }
  'debug:perf-trace-state-changed': { dir: 'main→renderer'; payload: PerfTraceState }
  'debug:perf-trace-toggle': { dir: 'invoke'; payload: unknown }
  'debug:reset-cursor-tuning': { dir: 'renderer→main'; payload: unknown }
  'debug:update-cursor-spline-viz': { dir: 'renderer→main'; payload: unknown }
  'debug:update-cursor-tuning': { dir: 'renderer→main'; payload: unknown }
  'devtools-changed': { dir: 'main→renderer'; payload: boolean }
  'devtools-resize-end': { dir: 'renderer→main'; payload: unknown }
  'devtools-resize-move': { dir: 'renderer→main'; payload: unknown }
  'devtools-resize-start': { dir: 'renderer→main'; payload: unknown }
  'dispatch-scroll': { dir: 'main→renderer'; payload: unknown }
  'dispatch-scroll-result': { dir: 'renderer→main'; payload: unknown }
  'fix-progress-update': { dir: 'main→renderer'; payload: LayoutUpdateData['fixProgress'] }
  'get-canvas-layout-bootstrap': { dir: 'invoke'; payload: unknown }
  'get-floating-ui-bootstrap': { dir: 'renderer→main'; payload: unknown }
  'get-left-sidebar-bootstrap': { dir: 'invoke'; payload: unknown }
  'get-theme-bootstrap': { dir: 'invoke'; payload: unknown }
  'inspect-focus-node': { dir: 'main→renderer'; payload: unknown }
  'inspect-node-detail-update': { dir: 'renderer→main'; payload: unknown }
  'inspect-node-hover': { dir: 'renderer→main'; payload: unknown }
  'inspect-node-select': { dir: 'renderer→main'; payload: unknown }
  'inspect-tree-update': { dir: 'renderer→main'; payload: unknown }
  'interaction-sync-event': { dir: 'renderer→main'; payload: InteractionSyncEvent }
  'left-sidebar-changed': { dir: 'main→renderer'; payload: boolean }
  'left-sidebar-data': { dir: 'main→renderer'; payload: LeftSidebarData }
  'onboarding:complete': { dir: 'renderer→main'; payload: unknown }
  'onboarding:dismiss': { dir: 'renderer→main'; payload: unknown }
  'onboarding:get-initial-data': { dir: 'invoke'; payload: unknown }
  'onboarding:install': { dir: 'invoke'; payload: unknown }
  'onboarding:progress': { dir: 'main→renderer'; payload: OnboardingProgressEvent }
  'onboarding:refresh-status': { dir: 'renderer→main'; payload: unknown }
  'override-props': { dir: 'main→renderer'; payload: unknown }
  'override-token': { dir: 'main→renderer'; payload: unknown }
  'page-annotations-update': { dir: 'main→renderer'; payload: unknown }
  'page-deselect': { dir: 'renderer→main'; payload: unknown }
  'page-hover': { dir: 'renderer→main'; payload: unknown }
  'page-scroll-changed': { dir: 'renderer→main'; payload: unknown }
  'peek-resize-end': { dir: 'renderer→main'; payload: unknown }
  'peek-resize-move': { dir: 'renderer→main'; payload: unknown }
  'peek-resize-start': { dir: 'renderer→main'; payload: unknown }
  'query-active-element-rect': { dir: 'main→renderer'; payload: unknown }
  'query-active-element-rect-result': { dir: 'renderer→main'; payload: unknown }
  'query-dom-elements': { dir: 'main→renderer'; payload: unknown }
  'query-dom-elements-response': { dir: 'renderer→main'; payload: unknown }
  'query-element-at-point': { dir: 'main→renderer'; payload: unknown }
  'query-element-at-point-response': { dir: 'renderer→main'; payload: unknown }
  'query-elements-in-rect': { dir: 'main→renderer'; payload: unknown }
  'query-elements-in-rect-response': { dir: 'renderer→main'; payload: unknown }
  'query-favicon': { dir: 'main→renderer'; payload: unknown }
  'query-favicon-result': { dir: 'renderer→main'; payload: unknown }
  'region-select-committed': { dir: 'main→renderer'; payload: { canvasRect: WorkspaceBounds } }
  'reload-app': { dir: 'renderer→main'; payload: unknown }
  'repo-bind-origin': { dir: 'invoke'; payload: unknown }
  'repo-changed': { dir: 'main→renderer'; payload: ConnectedRepo[] }
  'repo-connect': { dir: 'invoke'; payload: unknown }
  'repo-connect-via-picker': { dir: 'invoke'; payload: unknown }
  'repo-disconnect': { dir: 'invoke'; payload: unknown }
  'repo-find-for-path': { dir: 'renderer→main'; payload: unknown }
  'repo-list': { dir: 'renderer→main'; payload: unknown }
  'resolve-interaction-locator': { dir: 'main→renderer'; payload: LocatorResolveRequest }
  'resolve-interaction-locator-response': { dir: 'renderer→main'; payload: LocatorResolveResponse }
  'resolve-node-detail': { dir: 'main→renderer'; payload: unknown }
  'resolve-node-detail-response': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-clear-inspect-selection': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-create-annotation': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-data': { dir: 'main→renderer'; payload: DevtoolsPanelData }
  'right-details-panel-delete-annotation': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-delete-edge': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-delete-page': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-dismiss-browser-devtools': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-duplicate-page': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-edit-component-prop': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-edit-component-token': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-fix-single-annotation': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-hover-node': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-open-browser-devtools': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-pick-repo-for-origin': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-remove-origin-binding': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-reply-annotation': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-resolve-annotation': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-select-node': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-select-page': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-set-auto-fix': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-set-file-custom': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-set-file-preset': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-set-fix-config': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-set-page-color-scheme': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-set-page-preset': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-toggle-svg-device-shell': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-trigger-fix-comments': { dir: 'renderer→main'; payload: unknown }
  'right-details-panel-update-edge': { dir: 'renderer→main'; payload: unknown }
  'set-annotate-mode': { dir: 'main→renderer'; payload: unknown }
  'set-canvas-zoom': { dir: 'main→renderer'; payload: unknown }
  'set-design-system-manifest': { dir: 'main→renderer'; payload: unknown }
  'set-inspection-mode': { dir: 'main→renderer'; payload: unknown }
  'set-interaction-sync-capture': { dir: 'main→renderer'; payload: InteractionSyncCapturePayload }
  'set-interactive': { dir: 'main→renderer'; payload: unknown }
  'set-multi-selected': { dir: 'main→renderer'; payload: unknown }
  'set-show-all-nodes': { dir: 'main→renderer'; payload: unknown }
  'set-theme-mode': { dir: 'renderer→main'; payload: { mode: AppThemeMode } }
  'settings:close': { dir: 'renderer→main'; payload: unknown }
  'settings:fix-config-changed': { dir: 'main→renderer'; payload: FixConfig }
  'settings:get-initial-data': { dir: 'invoke'; payload: unknown }
  'settings:install-skills': { dir: 'invoke'; payload: unknown }
  'settings:refresh-status': { dir: 'invoke'; payload: unknown }
  'settings:remove-origin-binding': { dir: 'renderer→main'; payload: unknown }
  'settings:set-component-installed': { dir: 'invoke'; payload: unknown }
  'settings:set-fix-config': { dir: 'renderer→main'; payload: unknown }
  'settings:skill-progress': { dir: 'main→renderer'; payload: OnboardingProgressEvent }
  'take-dom-snapshot': { dir: 'main→renderer'; payload: unknown }
  'take-dom-snapshot-response': { dir: 'renderer→main'; payload: unknown }
  'toggle-devtools': { dir: 'renderer→main'; payload: unknown }
  'toggle-left-sidebar': { dir: 'renderer→main'; payload: unknown }
  'tool-defaults-set': { dir: 'renderer→main'; payload: unknown }
  'toolbar-dropdown-close': { dir: 'renderer→main'; payload: unknown }
  'toolbar-dropdown-open': { dir: 'renderer→main'; payload: unknown }
  'toolbar-selection-changed': { dir: 'main→renderer'; payload: ToolbarSelectionData }
  'toolbar-set-tool': { dir: 'renderer→main'; payload: unknown }
  'toolbar-tooltip-close': { dir: 'renderer→main'; payload: unknown }
  'toolbar-tooltip-open': { dir: 'renderer→main'; payload: unknown }
  'viewport-nudge': { dir: 'main→renderer'; payload: ViewportNudge }
  'apply-note-content': { dir: 'invoke'; payload: unknown }
  'write-note-file': { dir: 'invoke'; payload: unknown }
  'zoom-changed': { dir: 'main→renderer'; payload: number }
  'zoom-in': { dir: 'renderer→main'; payload: unknown }
  'zoom-out': { dir: 'renderer→main'; payload: unknown }
  'zoom-reset': { dir: 'renderer→main'; payload: unknown }
  'zoom-set': { dir: 'renderer→main'; payload: unknown }
}

export type IpcChannel = keyof IpcContract

export type IpcPayload<C extends IpcChannel> = IpcContract[C]['payload']

/** Channels a renderer sends up to main (`ipcRenderer.send`). */
export type RendererToMainChannel = {
  [C in IpcChannel]: IpcContract[C]['dir'] extends 'renderer→main' ? C : never
}[IpcChannel]

/**
 * Channel-name constants so senders reference the contract instead of restating
 * a raw literal. The `satisfies` clause keeps every value a declared channel.
 */
export const ipcChannels = {
  themeChanged: 'theme-changed',
  layoutUpdate: 'layout-update',
  aboveviewCursorUpdate: 'aboveview-cursor-update',
  agentPresenceChanged: 'agent-presence-changed',
  annotateClearHover: 'annotate-clear-hover',
  annotateElementSelected: 'annotate-element-selected',
  annotationBboxSubscriptions: 'annotation-bbox-subscriptions',
  annotationBboxUpdate: 'annotation-bbox-update',
  annotationLiveBbox: 'annotation-live-bbox',
  annotationOpenThread: 'annotation-open-thread',
  annotationThreadOpen: 'annotation-thread-open',
  applyLinkedScroll: 'apply-linked-scroll',
  applyNoteContent: 'apply-note-content',
  applyPageOverrides: 'apply-page-overrides',
  bindingFire: 'binding-fire',
  canvasBackPage: 'canvas-back-page',
  canvasBgDropdownClose: 'canvas-bg-dropdown-close',
  canvasBgDropdownOpen: 'canvas-bg-dropdown-open',
  canvasCancelEntityEdit: 'canvas-cancel-entity-edit',
  canvasClearAnnotateHover: 'canvas-clear-annotate-hover',
  canvasCommentClickAt: 'canvas-comment-click-at',
  canvasCommitEntityEdit: 'canvas-commit-entity-edit',
  canvasCommitRegionSelect: 'canvas-commit-region-select',
  canvasCopyFileAsPng: 'canvas-copy-file-as-png',
  canvasCopySelection: 'canvas-copy-selection',
  canvasCreateAnnotation: 'canvas-create-annotation',
  canvasCreateDrawing: 'canvas-create-drawing',
  canvasCreateRegionAnnotation: 'canvas-create-region-annotation',
  canvasCreateTab: 'canvas-create-tab',
  canvasDeleteDrawingEntity: 'canvas-delete-drawing-entity',
  canvasDeleteEdge: 'canvas-delete-edge',
  canvasUpdateEdge: 'canvas-update-edge',
  canvasDeleteEntity: 'canvas-delete-entity',
  canvasDeleteFileEntity: 'canvas-delete-file-entity',
  canvasDeleteGroup: 'canvas-delete-group',
  canvasDeletePage: 'canvas-delete-page',
  canvasDeleteSelection: 'canvas-delete-selection',
  canvasDeleteShape: 'canvas-delete-shape',
  canvasDeleteTab: 'canvas-delete-tab',
  canvasDeleteTextEntity: 'canvas-delete-text-entity',
  canvasArrangeSelection: 'canvas-arrange-selection',
  canvasDragCopyGroup: 'canvas-drag-copy-group',
  canvasDragCopySelection: 'canvas-drag-copy-selection',
  canvasDragEntity: 'canvas-drag-entity',
  canvasDragEntityEnd: 'canvas-drag-entity-end',
  canvasDragEntityStart: 'canvas-drag-entity-start',
  canvasDragGroup: 'canvas-drag-group',
  canvasDragGroupEnd: 'canvas-drag-group-end',
  canvasDragGroupStart: 'canvas-drag-group-start',
  canvasDragPage: 'canvas-drag-page',
  canvasDragPageEnd: 'canvas-drag-page-end',
  canvasDragPageStart: 'canvas-drag-page-start',
  canvasDragPreview: 'canvas-drag-preview',
  canvasDropComponentPath: 'canvas-drop-component-path',
  canvasDropFileBuffer: 'canvas-drop-file-buffer',
  canvasDuplicateDrawingEntity: 'canvas-duplicate-drawing-entity',
  canvasDuplicateFileEntity: 'canvas-duplicate-file-entity',
  canvasDuplicateGroup: 'canvas-duplicate-group',
  canvasDuplicatePage: 'canvas-duplicate-page',
  canvasDuplicateSelection: 'canvas-duplicate-selection',
  canvasDuplicateShape: 'canvas-duplicate-shape',
  canvasDuplicateTextEntity: 'canvas-duplicate-text-entity',
  canvasEdgeDragBegin: 'canvas-edge-drag-begin',
  canvasEdgeDragCancel: 'canvas-edge-drag-cancel',
  canvasEdgeDragCommit: 'canvas-edge-drag-commit',
  canvasEdgeDragTargetChange: 'canvas-edge-drag-target-change',
  canvasEdgeEditCommit: 'canvas-edge-edit-commit',
  canvasEdgeEditDiscard: 'canvas-edge-edit-discard',
  canvasEditComponentProp: 'canvas-edit-component-prop',
  canvasEditComponentToken: 'canvas-edit-component-token',
  canvasEnterGroup: 'canvas-enter-group',
  canvasEnterPageInteractive: 'canvas-enter-page-interactive',
  canvasFocusSelection: 'canvas-focus-selection',
  canvasForwardPage: 'canvas-forward-page',
  canvasForwardPointer: 'canvas-forward-pointer',
  canvasForwardWheel: 'canvas-forward-wheel',
  canvasGapResizeCancel: 'canvas-gap-resize-cancel',
  canvasGapResizeCommit: 'canvas-gap-resize-commit',
  canvasGapResizeMove: 'canvas-gap-resize-move',
  canvasGapResizeStart: 'canvas-gap-resize-start',
  canvasGroupSelection: 'canvas-group-selection',
  canvasGuides: 'canvas-guides',
  canvasHoverPage: 'canvas-hover-page',
  canvasMultiResizeBegin: 'canvas-multi-resize-begin',
  canvasMultiResizeEnd: 'canvas-multi-resize-end',
  canvasNavigatePage: 'canvas-navigate-page',
  canvasOpenDevtoolsSelection: 'canvas-open-devtools-selection',
  canvasPan: 'canvas-pan',
  canvasPasteSelection: 'canvas-paste-selection',
  canvasPlacePendingEntity: 'canvas-place-pending-entity',
  canvasReloadPage: 'canvas-reload-page',
  canvasRenameDrawingEntity: 'canvas-rename-drawing-entity',
  canvasRenameFileEntity: 'canvas-rename-file-entity',
  canvasRenameGroup: 'canvas-rename-group',
  canvasRenamePage: 'canvas-rename-page',
  canvasRenameTab: 'canvas-rename-tab',
  canvasRenameTextEntity: 'canvas-rename-text-entity',
  canvasReorderCancel: 'canvas-reorder-cancel',
  canvasReorderCommit: 'canvas-reorder-commit',
  canvasReorderMove: 'canvas-reorder-move',
  canvasReorderSidebarItem: 'canvas-reorder-sidebar-item',
  canvasReorderStack: 'canvas-reorder-stack',
  canvasReorderStart: 'canvas-reorder-start',
  canvasReorderTab: 'canvas-reorder-tab',
  canvasRequestEntityEdit: 'canvas-request-entity-edit',
  canvasResizeBegin: 'canvas-resize-begin',
  canvasResizeEnd: 'canvas-resize-end',
  canvasResizeMultiSelection: 'canvas-resize-multi-selection',
  canvasRestoreFocusCamera: 'canvas-restore-focus-camera',
  canvasRevealEntity: 'canvas-reveal-entity',
  canvasRevealGroup: 'canvas-reveal-group',
  canvasRevealPage: 'canvas-reveal-page',
  canvasSelectEdge: 'canvas-select-edge',
  canvasSelectEntities: 'canvas-select-entities',
  canvasSelectEntity: 'canvas-select-entity',
  canvasSelectGroup: 'canvas-select-group',
  canvasSelectInRect: 'canvas-select-in-rect',
  canvasSelectPage: 'canvas-select-page',
  canvasSelectTab: 'canvas-select-tab',
  canvasSelectionOverlay: 'canvas-selection-overlay',
  canvasSetAnnotationState: 'canvas-set-annotation-state',
  canvasSetDeviceOrientation: 'canvas-set-device-orientation',
  canvasSetFileDeviceOrientation: 'canvas-set-file-device-orientation',
  canvasSetFocusAnnotationsVisible: 'canvas-set-focus-annotations-visible',
  canvasSetFocusPresentationMode: 'canvas-set-focus-presentation-mode',
  canvasSetPageColorScheme: 'canvas-set-page-color-scheme',
  canvasSetPageCustom: 'canvas-set-page-custom',
  canvasSetPagePreset: 'canvas-set-page-preset',
  canvasSetSelectionPreset: 'canvas-set-selection-preset',
  canvasSetTextEditing: 'canvas-set-text-editing',
  canvasShowFileInFinder: 'canvas-show-file-in-finder',
  canvasShowPageContextMenu: 'canvas-show-page-context-menu',
  canvasToggleAnnotateMode: 'canvas-toggle-annotate-mode',
  canvasToggleDeviceShell: 'canvas-toggle-device-shell',
  canvasToggleDrawMode: 'canvas-toggle-draw-mode',
  canvasToggleFileDeviceShell: 'canvas-toggle-file-device-shell',
  canvasToggleSyncSelection: 'canvas-toggle-sync-selection',
  canvasUnsyncPage: 'canvas-unsync-page',
  canvasUngroupGroup: 'canvas-ungroup-group',
  canvasUngroupSelection: 'canvas-ungroup-selection',
  canvasUpdateEntity: 'canvas-update-entity',
  canvasUpdatePageBounds: 'canvas-update-page-bounds',
  canvasZoom: 'canvas-zoom',
  captureMode: 'capture-mode',
  commentCanvasPointCommitted: 'comment-canvas-point-committed',
  commentOverlaySetActive: 'comment-overlay-set-active',
  commentToolBboxSubscriptions: 'comment-tool-bbox-subscriptions',
  commentToolPagePreview: 'comment-tool-page-preview',
  commentToolPointerState: 'comment-tool-pointer-state',
  cursorSplineVizChanged: 'cursor-spline-viz-changed',
  debugLog: 'debug-log',
  debugGetInitialData: 'debug:get-initial-data',
  debugPerfPanZoomGetState: 'debug:perf-pan-zoom-get-state',
  debugPerfPanZoomRun: 'debug:perf-pan-zoom-run',
  debugPerfPanZoomStateChanged: 'debug:perf-pan-zoom-state-changed',
  debugPerfPanZoomStop: 'debug:perf-pan-zoom-stop',
  debugPerfTraceGetState: 'debug:perf-trace-get-state',
  debugPerfTraceGetSummary: 'debug:perf-trace-get-summary',
  debugPerfTraceList: 'debug:perf-trace-list',
  debugPerfTraceReveal: 'debug:perf-trace-reveal',
  debugPerfTraceStateChanged: 'debug:perf-trace-state-changed',
  debugPerfTraceToggle: 'debug:perf-trace-toggle',
  debugResetCursorTuning: 'debug:reset-cursor-tuning',
  debugUpdateCursorSplineViz: 'debug:update-cursor-spline-viz',
  debugUpdateCursorTuning: 'debug:update-cursor-tuning',
  devtoolsChanged: 'devtools-changed',
  devtoolsResizeEnd: 'devtools-resize-end',
  devtoolsResizeMove: 'devtools-resize-move',
  devtoolsResizeStart: 'devtools-resize-start',
  dispatchScroll: 'dispatch-scroll',
  dispatchScrollResult: 'dispatch-scroll-result',
  fixProgressUpdate: 'fix-progress-update',
  getCanvasLayoutBootstrap: 'get-canvas-layout-bootstrap',
  getFloatingUiBootstrap: 'get-floating-ui-bootstrap',
  getLeftSidebarBootstrap: 'get-left-sidebar-bootstrap',
  getThemeBootstrap: 'get-theme-bootstrap',
  inspectFocusNode: 'inspect-focus-node',
  inspectNodeDetailUpdate: 'inspect-node-detail-update',
  inspectNodeHover: 'inspect-node-hover',
  inspectNodeSelect: 'inspect-node-select',
  inspectTreeUpdate: 'inspect-tree-update',
  interactionSyncEvent: 'interaction-sync-event',
  leftSidebarChanged: 'left-sidebar-changed',
  leftSidebarData: 'left-sidebar-data',
  onboardingComplete: 'onboarding:complete',
  onboardingDismiss: 'onboarding:dismiss',
  onboardingGetInitialData: 'onboarding:get-initial-data',
  onboardingInstall: 'onboarding:install',
  onboardingProgress: 'onboarding:progress',
  onboardingRefreshStatus: 'onboarding:refresh-status',
  overrideProps: 'override-props',
  overrideToken: 'override-token',
  pageAnnotationsUpdate: 'page-annotations-update',
  pageDeselect: 'page-deselect',
  pageScrollChanged: 'page-scroll-changed',
  peekResizeEnd: 'peek-resize-end',
  peekResizeMove: 'peek-resize-move',
  peekResizeStart: 'peek-resize-start',
  queryActiveElementRect: 'query-active-element-rect',
  queryActiveElementRectResult: 'query-active-element-rect-result',
  queryDomElements: 'query-dom-elements',
  queryDomElementsResponse: 'query-dom-elements-response',
  queryElementAtPoint: 'query-element-at-point',
  queryElementAtPointResponse: 'query-element-at-point-response',
  queryElementsInRect: 'query-elements-in-rect',
  queryElementsInRectResponse: 'query-elements-in-rect-response',
  queryFavicon: 'query-favicon',
  queryFaviconResult: 'query-favicon-result',
  regionSelectCommitted: 'region-select-committed',
  reloadApp: 'reload-app',
  repoBindOrigin: 'repo-bind-origin',
  repoChanged: 'repo-changed',
  repoConnect: 'repo-connect',
  repoConnectViaPicker: 'repo-connect-via-picker',
  repoDisconnect: 'repo-disconnect',
  repoFindForPath: 'repo-find-for-path',
  repoList: 'repo-list',
  resolveInteractionLocator: 'resolve-interaction-locator',
  resolveInteractionLocatorResponse: 'resolve-interaction-locator-response',
  resolveNodeDetail: 'resolve-node-detail',
  resolveNodeDetailResponse: 'resolve-node-detail-response',
  rightDetailsPanelClearInspectSelection: 'right-details-panel-clear-inspect-selection',
  rightDetailsPanelCreateAnnotation: 'right-details-panel-create-annotation',
  rightDetailsPanelData: 'right-details-panel-data',
  rightDetailsPanelDeleteAnnotation: 'right-details-panel-delete-annotation',
  rightDetailsPanelDeleteEdge: 'right-details-panel-delete-edge',
  rightDetailsPanelDeletePage: 'right-details-panel-delete-page',
  rightDetailsPanelDismissBrowserDevtools: 'right-details-panel-dismiss-browser-devtools',
  rightDetailsPanelDuplicatePage: 'right-details-panel-duplicate-page',
  rightDetailsPanelEditComponentProp: 'right-details-panel-edit-component-prop',
  rightDetailsPanelEditComponentToken: 'right-details-panel-edit-component-token',
  rightDetailsPanelFixSingleAnnotation: 'right-details-panel-fix-single-annotation',
  rightDetailsPanelHoverNode: 'right-details-panel-hover-node',
  rightDetailsPanelOpenBrowserDevtools: 'right-details-panel-open-browser-devtools',
  rightDetailsPanelPickRepoForOrigin: 'right-details-panel-pick-repo-for-origin',
  rightDetailsPanelRemoveOriginBinding: 'right-details-panel-remove-origin-binding',
  rightDetailsPanelReplyAnnotation: 'right-details-panel-reply-annotation',
  rightDetailsPanelResolveAnnotation: 'right-details-panel-resolve-annotation',
  rightDetailsPanelSelectNode: 'right-details-panel-select-node',
  rightDetailsPanelSelectPage: 'right-details-panel-select-page',
  rightDetailsPanelSetAutoFix: 'right-details-panel-set-auto-fix',
  rightDetailsPanelSetFileCustom: 'right-details-panel-set-file-custom',
  rightDetailsPanelSetFilePreset: 'right-details-panel-set-file-preset',
  rightDetailsPanelSetFixConfig: 'right-details-panel-set-fix-config',
  rightDetailsPanelSetPageColorScheme: 'right-details-panel-set-page-color-scheme',
  rightDetailsPanelSetPagePreset: 'right-details-panel-set-page-preset',
  rightDetailsPanelToggleSvgDeviceShell: 'right-details-panel-toggle-svg-device-shell',
  rightDetailsPanelTriggerFixComments: 'right-details-panel-trigger-fix-comments',
  rightDetailsPanelUpdateEdge: 'right-details-panel-update-edge',
  setAnnotateMode: 'set-annotate-mode',
  setCanvasZoom: 'set-canvas-zoom',
  setDesignSystemManifest: 'set-design-system-manifest',
  setInspectionMode: 'set-inspection-mode',
  setInteractionSyncCapture: 'set-interaction-sync-capture',
  setInteractive: 'set-interactive',
  setMultiSelected: 'set-multi-selected',
  setThemeMode: 'set-theme-mode',
  settingsClose: 'settings:close',
  settingsFixConfigChanged: 'settings:fix-config-changed',
  settingsGetInitialData: 'settings:get-initial-data',
  settingsInstallSkills: 'settings:install-skills',
  settingsRefreshStatus: 'settings:refresh-status',
  settingsRemoveOriginBinding: 'settings:remove-origin-binding',
  settingsSetComponentInstalled: 'settings:set-component-installed',
  settingsSetFixConfig: 'settings:set-fix-config',
  settingsSkillProgress: 'settings:skill-progress',
  takeDomSnapshot: 'take-dom-snapshot',
  takeDomSnapshotResponse: 'take-dom-snapshot-response',
  toggleDevtools: 'toggle-devtools',
  toggleLeftSidebar: 'toggle-left-sidebar',
  toolDefaultsSet: 'tool-defaults-set',
  toolbarDropdownClose: 'toolbar-dropdown-close',
  toolbarDropdownOpen: 'toolbar-dropdown-open',
  toolbarSelectionChanged: 'toolbar-selection-changed',
  toolbarSetTool: 'toolbar-set-tool',
  toolbarTooltipClose: 'toolbar-tooltip-close',
  toolbarTooltipOpen: 'toolbar-tooltip-open',
  viewportNudge: 'viewport-nudge',
  writeNoteFile: 'write-note-file',
  zoomChanged: 'zoom-changed',
  zoomIn: 'zoom-in',
  zoomOut: 'zoom-out',
  zoomReset: 'zoom-reset',
  zoomSet: 'zoom-set',
} as const satisfies Record<string, IpcChannel>
