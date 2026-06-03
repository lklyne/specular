import { useEffect, useState } from 'react'
import { Copy, LayoutGrid, Trash2 } from 'lucide-react'
import type { PanelFileEntityDetail, WireframePanelSelection } from '../../../shared/types'
import {
  editorDisplayValue,
  editorsForNodeType,
  patchForEditorChange,
  type WireframePropEditor,
} from '../../../shared/wireframe/wireframe-prop-editors'
import { paneDeleteBtnClass as deleteBtnClass, dividerClass, paneActionBtnClass as iconBtnClass, mutedClass } from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { FileDeviceSection } from './FileDeviceSection'
import { PaneHeader } from './PaneHeader'

const THEME_OPTIONS = [
  { value: 'light', label: 'Light', color: '#ffffff' },
  { value: 'dark', label: 'Dark', color: '#18181b' },
  { value: 'blueprint', label: 'Blueprint', color: '#0f2744' },
] as const

const NODE_TYPE_ICONS: Record<string, string> = {
  page: '□',
  text: 'T',
  button: '⊞',
  input: '▭',
  dropdown: '▾',
  checkbox: '☐',
  toggle: '◑',
  image: '⊟',
  divider: '―',
  spacer: '⋯',
}

interface WireframeNode {
  id: string
  type: string
  text?: string
  label?: string
  placeholder?: string
  children?: WireframeNode[]
  direction?: string
  [key: string]: unknown
}

function nodeLabel(node: WireframeNode): string {
  if (node.type === 'text' && node.text) return node.text.slice(0, 30)
  if (node.type === 'button' && node.text) return node.text
  if (node.type === 'input') return node.label ?? node.placeholder ?? 'Input'
  if (node.type === 'dropdown') return node.label ?? node.placeholder ?? 'Dropdown'
  if (node.type === 'checkbox' || node.type === 'toggle') return node.label ?? node.type
  if (node.type === 'page') {
    const dir = node.direction === 'horizontal' ? '→' : '↓'
    return `Page ${dir}`
  }
  return node.type
}

function NodeTreeItem({
  node,
  depth,
  isDark,
}: {
  node: WireframeNode
  depth: number
  isDark: boolean
}) {
  const icon = NODE_TYPE_ICONS[node.type] ?? '?'
  const muted = isDark ? 'text-zinc-500' : 'text-zinc-400'

  return (
    <>
      <div
        className={`flex items-center gap-1.5 py-0.5 pr-2 text-[11px] ${
          isDark ? 'hover:bg-zinc-800' : 'hover:bg-zinc-100'
        }`}
        style={{ paddingLeft: 8 + depth * 12 }}
      >
        <span className={`font-mono text-[10px] ${muted}`} style={{ width: 12, textAlign: 'center' }}>
          {icon}
        </span>
        <span className="min-w-0 flex-1 truncate">{nodeLabel(node)}</span>
        <span className={`text-[9px] ${muted}`}>{node.id}</span>
      </div>
      {node.children?.map((child) => (
        <NodeTreeItem key={child.id} node={child} depth={depth + 1} isDark={isDark} />
      ))}
    </>
  )
}

const PALETTE_ITEMS = [
  { type: 'page', label: 'Page' },
  { type: 'text', label: 'Text' },
  { type: 'button', label: 'Button' },
  { type: 'input', label: 'Input' },
  { type: 'dropdown', label: 'Dropdown' },
  { type: 'checkbox', label: 'Checkbox' },
  { type: 'toggle', label: 'Toggle' },
  { type: 'image', label: 'Image' },
  { type: 'divider', label: 'Divider' },
  { type: 'spacer', label: 'Spacer' },
]

function fieldClass(isDark: boolean): string {
  return isDark
    ? 'w-full rounded border border-zinc-700 bg-zinc-800 px-1.5 py-1 text-[11px] text-zinc-200 outline-none focus:border-zinc-500'
    : 'w-full rounded border border-zinc-300 bg-white px-1.5 py-1 text-[11px] text-zinc-700 outline-none focus:border-zinc-400'
}

// Commits on blur and Enter, re-seeding from upstream when the broadcast value
// changes (e.g. an edit round-trips back). Used for text / number / options /
// fixed-px controls so typing isn't fired per-keystroke.
function CommitInput({
  value,
  type,
  isDark,
  onCommit,
}: {
  value: string
  type: 'text' | 'number'
  isDark: boolean
  onCommit: (raw: string) => void
}) {
  const [draft, setDraft] = useState(value)
  useEffect(() => setDraft(value), [value])
  return (
    <input
      type={type}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={() => {
        if (draft !== value) onCommit(draft)
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter') e.currentTarget.blur()
      }}
      className={fieldClass(isDark)}
    />
  )
}

const SIZING_MODES = [
  { value: 'hug', label: 'Hug' },
  { value: 'fill', label: 'Fill' },
  { value: 'fixed', label: 'Fixed' },
] as const

function PropEditorRow({
  node,
  editor,
  entityId,
  isDark,
}: {
  node: WireframePanelSelection['node']
  editor: WireframePropEditor
  entityId: string
  isDark: boolean
}) {
  const value = editorDisplayValue(node, editor)
  const muted = mutedClass(isDark)
  const commit = (raw: string) => {
    const patch = patchForEditorChange(editor, raw)
    if (patch) rightDetailsPanelApi.updateWireframeNodeProps(entityId, node.id, patch)
  }

  let control: React.ReactNode
  if (editor.control === 'select') {
    control = (
      <select
        value={value}
        onChange={(e) => commit(e.target.value)}
        className={fieldClass(isDark)}
      >
        {editor.options?.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    )
  } else if (editor.control === 'sizing') {
    const mode = value === 'fill' ? 'fill' : value === 'hug' || value === '' ? 'hug' : 'fixed'
    control = (
      <div className="flex items-center gap-1">
        <select
          value={mode}
          onChange={(e) => {
            const next = e.target.value
            if (next === 'fixed') commit(/^\d/.test(value) ? value : '100')
            else commit(next)
          }}
          className={fieldClass(isDark)}
        >
          {SIZING_MODES.map((m) => (
            <option key={m.value} value={m.value}>
              {m.label}
            </option>
          ))}
        </select>
        {mode === 'fixed' ? (
          <CommitInput value={value} type="number" isDark={isDark} onCommit={commit} />
        ) : null}
      </div>
    )
  } else {
    control = (
      <CommitInput
        value={value}
        type={editor.control === 'number' ? 'number' : 'text'}
        isDark={isDark}
        onCommit={commit}
      />
    )
  }

  return (
    <label className="flex items-center gap-2">
      <span className={`w-16 shrink-0 text-[10px] ${muted}`}>{editor.label}</span>
      <span className="min-w-0 flex-1">{control}</span>
    </label>
  )
}

function SelectedNodeSection({
  selection,
  isDark,
  divider,
  muted,
}: {
  selection: WireframePanelSelection
  isDark: boolean
  divider: string
  muted: string
}) {
  const { node } = selection
  const editors = editorsForNodeType(node.type)
  const icon = NODE_TYPE_ICONS[node.type] ?? '?'
  return (
    <div className={`border-t px-2 pt-2 pb-2 ${divider}`}>
      <div className={`mb-1.5 flex items-center gap-1.5 text-[10px] font-medium ${muted}`}>
        <span className="font-mono text-[10px]">{icon}</span>
        <span className="capitalize">{node.type}</span>
        <span className="text-[9px] opacity-70">{node.id}</span>
      </div>
      {editors.length === 0 ? (
        <div className={`text-[11px] ${muted}`}>No editable properties</div>
      ) : (
        <div className="flex flex-col gap-1.5">
          {editors.map((editor) => (
            <PropEditorRow
              key={editor.key}
              node={node}
              editor={editor}
              entityId={selection.entityId}
              isDark={isDark}
            />
          ))}
        </div>
      )}
    </div>
  )
}

export function WireframeFilePane({
  fileEntity,
  isDark,
  wireframeSelection,
}: {
  fileEntity: PanelFileEntityDetail
  isDark: boolean
  wireframeSelection?: WireframePanelSelection
}) {
  const muted = mutedClass(isDark)
  const divider = dividerClass(isDark)
  const fileName = fileEntity.file.split('/').pop()?.replace(/\.wireframe\.json$/i, '') ?? 'Wireframe'

  return (
    <div className="flex flex-col">
      <PaneHeader
        icon={<LayoutGrid size={14} className="shrink-0 text-zinc-500" />}
        label={fileName}
        actions={
          <>
            <button
              type="button"
              className={iconBtnClass(isDark)}
              onClick={() => rightDetailsPanelApi.duplicateFileEntity(fileEntity.id)}
              title="Duplicate"
            >
              <Copy size={14} />
            </button>
            <button
              type="button"
              className={deleteBtnClass(isDark)}
              onClick={() => rightDetailsPanelApi.deleteFileEntity(fileEntity.id)}
              title="Delete"
            >
              <Trash2 size={14} />
            </button>
          </>
        }
      />

      <FileDeviceSection fileEntity={fileEntity} isDark={isDark} divider={divider} />

      {wireframeSelection && wireframeSelection.entityId === fileEntity.id ? (
        <SelectedNodeSection
          selection={wireframeSelection}
          isDark={isDark}
          divider={divider}
          muted={muted}
        />
      ) : null}

      <div className={`border-t px-2 pt-2 pb-2 ${divider}`}>
        <div className={`mb-1.5 text-[10px] font-medium ${muted}`}>Theme</div>
        <div className="flex items-center gap-1.5">
          {THEME_OPTIONS.map((t) => (
            <div
              key={t.value}
              className="flex items-center gap-1"
              title={t.label}
            >
              <span
                className={`block h-3.5 w-3.5 rounded-full border ${
                  isDark ? 'border-zinc-600' : 'border-zinc-300'
                }`}
                style={{ background: t.color }}
              />
            </div>
          ))}
          <span className={`ml-1 text-[10px] ${muted}`}>
            Edit on canvas
          </span>
        </div>
      </div>

      {/* Component palette — insert a node into the root frame */}
      <div className={`border-t px-2 pt-2 pb-2 ${divider}`}>
        <div className={`mb-1.5 text-[10px] font-medium ${muted}`}>Components</div>
        <div className="grid grid-cols-2 gap-1">
          {PALETTE_ITEMS.map((item) => (
            <button
              key={item.type}
              type="button"
              onClick={() => rightDetailsPanelApi.insertWireframeNode(fileEntity.id, item.type)}
              className={`flex items-center gap-1.5 rounded px-2 py-1 text-left text-[11px] ${
                isDark
                  ? 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'
                  : 'bg-zinc-100 text-zinc-600 hover:bg-zinc-200'
              }`}
              title={`Add ${item.label.toLowerCase()}`}
            >
              <span className="font-mono text-[10px] opacity-60" style={{ width: 12, textAlign: 'center' }}>
                {NODE_TYPE_ICONS[item.type]}
              </span>
              {item.label}
            </button>
          ))}
        </div>
      </div>

      {/* Dimensions */}
      <div className={`border-t px-2 pt-2 pb-2 ${divider}`}>
        <div className={`mb-1 text-[10px] font-medium ${muted}`}>Dimensions</div>
        <div className={`text-[11px] ${muted}`}>
          {fileEntity.width} × {fileEntity.height}
        </div>
      </div>

      {/* Path */}
      <div className={`border-t px-2 pt-2 pb-2 ${divider}`}>
        <div className={`mb-1 text-[10px] font-medium ${muted}`}>Path</div>
        <div
          className={`break-all rounded px-2 py-1.5 text-[11px] leading-5 ${
            isDark ? 'bg-zinc-800' : 'bg-zinc-100'
          }`}
          title={fileEntity.file}
        >
          {fileEntity.file}
        </div>
      </div>
    </div>
  )
}
