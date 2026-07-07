import { ZoomPresetDropdown } from '../shared/ZoomPresetDropdown'
import {
  ChevronLeft,
  ChevronRight,
  PanelRight,
  RotateCw,
} from 'lucide-react'
import type {
  AgentPresenceCursor,
  DrawingBrushType,
  Tool,
} from '../../shared/types'
import { summarizePresenceCursor } from '../../shared/agent-presence'
import { shortcutDisplay } from '../../shared/bindings'
import { resolveCanvasColor } from '../../shared/canvas-colors'
import {
  AddPageToolIcon,
  AddDocumentToolIcon,
  AddShapeToolIcon,
  AddStickyToolIcon,
  AddTextToolIcon,
  CommentToolIcon,
  DrawHighlightToolIcon,
  DrawPenToolIcon,
  HandToolIcon,
  InspectToolIcon,
  SelectToolIcon,
  ThemeToolIcon,
  ZoomChevronIcon,
} from '../shared/CustomIcons'
import { ToolbarTooltip } from './ToolbarTooltip'
import { ZOOM_PRESETS } from './useToolbarState'

function toolbarIconBtnClass(isDark: boolean): string {
  return isDark
    ? 'tb-hit toolbar-squircle-btn rounded-[8px] border border-transparent bg-transparent p-1.5 text-zinc-300 hover:bg-[var(--surface-interactive-hover)] hover:text-zinc-100 active:bg-[var(--surface-interactive)] disabled:pointer-events-none disabled:opacity-45'
    : 'tb-hit toolbar-squircle-btn rounded-[8px] border border-transparent bg-transparent p-1.5 text-zinc-600 hover:bg-[var(--surface-interactive-hover)] hover:text-zinc-900 active:bg-[var(--surface-interactive)] disabled:pointer-events-none disabled:opacity-45'
}

// Tool buttons in the central toolbar follow the Figma toolbar spec:
// 32×28 container, radius 6, single fill drives hover & active. Larger than
// the popup IconButton (ADR 0013 §8, 24×24) — the toolbar is the primary
// surface and its glyphs need to read at a glance.
function toolbarToolBtnClass(isDark: boolean, active: boolean): string {
  const base =
    'tb-hit flex h-7 w-8 items-center justify-center rounded-[6px] border-0 transition-colors disabled:pointer-events-none disabled:opacity-45'
  if (active) {
    return isDark
      ? `${base} bg-[rgba(253,248,245,0.1)] text-zinc-100`
      : `${base} bg-[#fdf8f5] text-zinc-900`
  }
  return isDark
    ? `${base} text-zinc-300 hover:bg-[rgba(253,248,245,0.1)] hover:text-zinc-100`
    : `${base} text-zinc-600 hover:bg-[#fdf8f5] hover:text-zinc-900`
}

// Toolbar icon glyphs render at 20px wide per the Figma spec; the largest
// natural-aspect asset (29×27 add-page) sits comfortably inside the 32×28 button.
const TOOL_GLYPH_SIZE = 20

// Light and dark glyphs ship as parallel SVG assets — see `makeToolbarIcon`
// in CustomIcons.tsx, which picks the right URL from `isDark`. CSS only
// applies the drop-shadow on top; we no longer invert the light asset for
// dark mode because that pushed the light-grey gradient to near-black and
// looked muddy against the dark toolbar.
const TOOLBAR_GLYPH_SHADOW = 'drop-shadow(0 1px 1.5px rgba(0, 0, 0, 0.18))'
const TOOLBAR_GLYPH_STYLE: React.CSSProperties = { filter: TOOLBAR_GLYPH_SHADOW }

export function ToolbarDivider({ isDark }: { isDark: boolean }) {
  return (
    <div
      className={`mx-1 h-4 w-px shrink-0 ${isDark ? 'bg-white/20' : 'bg-zinc-900/20'}`}
    />
  )
}

interface LeftActionsProps {
  isDark: boolean
  leftSidebarOpen: boolean
  onToggleLeftSidebar: () => void
}

export function LeftActions({
  isDark,
  leftSidebarOpen,
  onToggleLeftSidebar,
}: LeftActionsProps) {
  const iconButtonClassName = toolbarIconBtnClass(isDark)
  const label = leftSidebarOpen ? 'Collapse left panel' : 'Expand left panel'

  return (
    <div className="flex min-w-0 items-center justify-start">
      <div className="flex w-fit items-center gap-2 [-webkit-app-region:no-drag]">
        <ToolbarTooltip label={label}>
          <button
            onClick={onToggleLeftSidebar}
            className={iconButtonClassName}
            aria-label={label}
            type="button"
          >
            <PanelRight
              size={14}
              className={leftSidebarOpen ? '' : 'opacity-60'}
              style={{ transform: 'scaleX(-1)' }}
            />
          </button>
        </ToolbarTooltip>
      </div>
    </div>
  )
}

interface CenterActionsProps {
  isDark: boolean
  activeTool: Tool
  drawBrushType: DrawingBrushType
  drawColor: string
  stickyColor: string
  shapeColor: string
  hasSelection: boolean
  zoomPercent: number
  currentPresetValue: (typeof ZOOM_PRESETS)[number] | null
  onSetTool: (tool: Tool) => void
  onDropdownOpenChange: (open: boolean) => void
  onToggleTheme: () => void
  onZoomSet: (value: number) => void
}

export function CenterActions({
  isDark,
  activeTool,
  drawBrushType,
  drawColor,
  stickyColor,
  shapeColor,
  hasSelection,
  zoomPercent,
  currentPresetValue,
  onSetTool,
  onDropdownOpenChange,
  onToggleTheme,
  onZoomSet,
}: CenterActionsProps) {
  const onAddPage = () => onSetTool({ kind: 'add-page' })
  const onSelectTool = () => onSetTool({ kind: 'select' })
  const onToggleHandTool = () =>
    onSetTool(activeTool.kind === 'hand' ? { kind: 'select' } : { kind: 'hand' })
  const onToggleDrawMode = () =>
    onSetTool(activeTool.kind === 'draw' ? { kind: 'select' } : { kind: 'draw' })
  const onAddSticky = () => onSetTool({ kind: 'add-sticky' })
  const onAddShape = () => onSetTool({ kind: 'add-shape' })
  const onAddText = () => onSetTool({ kind: 'add-text' })
  const onAddDocument = () => onSetTool({ kind: 'add-document' })
  const onToggleCommentMode = () =>
    onSetTool(activeTool.kind === 'comment' ? { kind: 'select' } : { kind: 'comment' })
  const onToggleInspectMode = () =>
    onSetTool(activeTool.kind === 'inspect' ? { kind: 'select' } : { kind: 'inspect' })

  const isMac = navigator.userAgent.includes('Mac')
  const sc = (id: Parameters<typeof shortcutDisplay>[0]) => shortcutDisplay(id, isMac)
  const buttonClass = (active: boolean) => toolbarToolBtnClass(isDark, active)
  const drawInk = resolveCanvasColor(drawColor, {
    role: 'ink',
    isDark,
    palette: 'vivid',
  })
  const selectTriggerClassName = isDark
    ? 'toolbar-squircle-btn flex h-7 w-[58px] cursor-pointer items-center justify-between gap-0.5 rounded-[6px] border border-transparent bg-transparent pl-2 pr-1 text-xs tabular-nums text-zinc-200 hover:bg-[rgba(253,248,245,0.1)]'
    : 'toolbar-squircle-btn flex h-7 w-[58px] cursor-pointer items-center justify-between gap-0.5 rounded-[6px] border border-transparent bg-transparent pl-2 pr-1 text-xs tabular-nums text-zinc-600 hover:bg-[#fdf8f5] hover:text-zinc-900'
  // ADR 0013 §5 grouping: nav | create | annotate | view.
  return (
    <div className="flex min-w-0 items-center justify-center overflow-hidden">
      <div className="flex w-fit items-center gap-1 [-webkit-app-region:no-drag]">
        <ToolbarTooltip label="Select tool" shortcut={sc('tool-select')}>
          <button
            onClick={onSelectTool}
            className={buttonClass(activeTool.kind === 'select')}
            aria-label="Select tool"
            type="button"
          >
            <SelectToolIcon size={TOOL_GLYPH_SIZE} isDark={isDark} style={TOOLBAR_GLYPH_STYLE} />
          </button>
        </ToolbarTooltip>

        <ToolbarTooltip label="Hand tool" shortcut={sc('tool-hand')}>
          <button
            onClick={onToggleHandTool}
            className={buttonClass(activeTool.kind === 'hand')}
            aria-label="Hand tool"
            type="button"
          >
            <HandToolIcon size={TOOL_GLYPH_SIZE} isDark={isDark} style={TOOLBAR_GLYPH_STYLE} />
          </button>
        </ToolbarTooltip>

        <ToolbarDivider isDark={isDark} />

        <ToolbarTooltip label="Draw" shortcut={sc('tool-draw-pen')}>
          <button
            onClick={onToggleDrawMode}
            className={buttonClass(activeTool.kind === 'draw')}
            aria-label="Draw"
            type="button"
          >
            {drawBrushType === 'pen' ? (
              <DrawPenToolIcon
                size={TOOL_GLYPH_SIZE}
                isDark={isDark}
                ink={drawInk}
                style={TOOLBAR_GLYPH_STYLE}
              />
            ) : (
              <DrawHighlightToolIcon
                size={TOOL_GLYPH_SIZE}
                isDark={isDark}
                ink={drawInk}
                style={TOOLBAR_GLYPH_STYLE}
              />
            )}
          </button>
        </ToolbarTooltip>

        <ToolbarTooltip label="Add sticky" shortcut={sc('tool-add-sticky')}>
          <button
            onClick={onAddSticky}
            className={buttonClass(activeTool.kind === 'add-sticky')}
            aria-label="Add sticky"
            type="button"
          >
            <AddStickyToolIcon size={TOOL_GLYPH_SIZE} isDark={isDark} color={stickyColor} />
          </button>
        </ToolbarTooltip>

        <ToolbarTooltip label="Add shape" shortcut={sc('tool-add-shape-rectangle')}>
          <button
            onClick={onAddShape}
            className={buttonClass(activeTool.kind === 'add-shape')}
            aria-label="Add shape"
            type="button"
          >
            <AddShapeToolIcon
              size={TOOL_GLYPH_SIZE}
              isDark={isDark}
              color={shapeColor}
              style={TOOLBAR_GLYPH_STYLE}
            />
          </button>
        </ToolbarTooltip>

        <ToolbarTooltip label="Add page" shortcut={sc('tool-add-page')}>
          <button
            onClick={onAddPage}
            className={buttonClass(activeTool.kind === 'add-page')}
            aria-label="Add page"
            type="button"
          >
            <AddPageToolIcon
              size={TOOL_GLYPH_SIZE}
              isDark={isDark}
              style={TOOLBAR_GLYPH_STYLE}
            />
          </button>
        </ToolbarTooltip>

        <ToolbarTooltip label="Add text" shortcut={sc('tool-add-text')}>
          <button
            onClick={onAddText}
            className={buttonClass(activeTool.kind === 'add-text')}
            aria-label="Add text"
            type="button"
          >
            <AddTextToolIcon size={TOOL_GLYPH_SIZE} isDark={isDark} style={TOOLBAR_GLYPH_STYLE} />
          </button>
        </ToolbarTooltip>

        <ToolbarTooltip label="Add document">
          <button
            onClick={onAddDocument}
            className={buttonClass(activeTool.kind === 'add-document')}
            aria-label="Add document"
            type="button"
          >
            <AddDocumentToolIcon size={TOOL_GLYPH_SIZE} isDark={isDark} style={TOOLBAR_GLYPH_STYLE} />
          </button>
        </ToolbarTooltip>

        <ToolbarDivider isDark={isDark} />

        <ToolbarTooltip label="Comment" shortcut={sc('tool-comment')}>
          <button
            onClick={onToggleCommentMode}
            className={buttonClass(activeTool.kind === 'comment')}
            aria-label="Comment"
            type="button"
          >
            <CommentToolIcon size={TOOL_GLYPH_SIZE} isDark={isDark} style={TOOLBAR_GLYPH_STYLE} />
          </button>
        </ToolbarTooltip>

        <ToolbarTooltip label={hasSelection ? 'Inspect' : 'Inspect any page'} shortcut={sc('tool-inspect')}>
          <button
            onClick={onToggleInspectMode}
            className={buttonClass(activeTool.kind === 'inspect')}
            aria-label={hasSelection ? 'Inspect' : 'Inspect any page'}
            type="button"
          >
            <InspectToolIcon size={TOOL_GLYPH_SIZE} isDark={isDark} style={TOOLBAR_GLYPH_STYLE} />
          </button>
        </ToolbarTooltip>

        <ToolbarDivider isDark={isDark} />

        <ToolbarTooltip label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}>
          <button
            onClick={onToggleTheme}
            className={buttonClass(false)}
            aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
            type="button"
          >
            <ThemeToolIcon size={TOOL_GLYPH_SIZE} isDark={isDark} style={TOOLBAR_GLYPH_STYLE} />
          </button>
        </ToolbarTooltip>

        <ZoomPresetDropdown
          isDark={isDark}
          levels={ZOOM_PRESETS}
          activeLevel={currentPresetValue}
          shortcutLevel={100}
          onSelect={onZoomSet}
          onOpenChange={onDropdownOpenChange}
          trigger={
            <button type="button" data-zoom-anchor className={selectTriggerClassName} title="Zoom">
              <span>{zoomPercent}%</span>
              <ZoomChevronIcon size={10} isDark={isDark} style={TOOLBAR_GLYPH_STYLE} />
            </button>
          }
        />
      </div>
    </div>
  )
}

interface RightPanelToggleProps {
  isDark: boolean
  devtoolsOpen: boolean
  onToggleDevTools: () => void
}

export function RightPanelToggle({
  isDark,
  devtoolsOpen,
  onToggleDevTools,
}: RightPanelToggleProps) {
  const iconButtonClassName = toolbarIconBtnClass(isDark)
  const label = devtoolsOpen ? 'Collapse right panel' : 'Expand right panel'

  return (
    <div className="flex min-w-0 items-center justify-end">
      <div className="flex w-fit items-center gap-1 [-webkit-app-region:no-drag]">
        <ToolbarTooltip label={label}>
          <button
            onClick={onToggleDevTools}
            className={iconButtonClassName}
            aria-label={label}
            type="button"
          >
            <PanelRight size={14} className={devtoolsOpen ? '' : 'opacity-60'} />
          </button>
        </ToolbarTooltip>
      </div>
    </div>
  )
}

interface ToolbarStatusActionsProps {
  isDark: boolean
  agentCursors: AgentPresenceCursor[]
}

export function ToolbarStatusActions({
  isDark,
  agentCursors,
}: ToolbarStatusActionsProps) {
  const activeAgentCursors = agentCursors.filter((c) => c.activity !== 'idle')

  return (
    <>

      {activeAgentCursors.length > 0 ? (
        <div className="flex items-center gap-1 pr-1.5">
          {activeAgentCursors.slice(0, 3).map((c) => (
            <div
              key={c.sessionId}
              className="flex items-center justify-center rounded-full"
              title={summarizePresenceCursor(c) ?? c.clientName}
              style={{
                width: 20,
                height: 20,
                backgroundColor: c.color,
              }}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#fff"
                strokeWidth="2.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M12 8V4H8" />
                <rect width="16" height="12" x="4" y="8" rx="2" />
                <path d="M2 14h2" />
                <path d="M20 14h2" />
                <path d="M15 13v2">
                  <animate attributeName="d" values="M15 13v2;M15 12v3;M15 13v2" dur="1.5s" repeatCount="indefinite" />
                </path>
                <path d="M9 13v2">
                  <animate attributeName="d" values="M9 13v2;M9 12v3;M9 13v2" dur="1.5s" repeatCount="indefinite" begin="0.2s" />
                </path>
              </svg>
            </div>
          ))}
        </div>
      ) : null}
    </>
  )
}
