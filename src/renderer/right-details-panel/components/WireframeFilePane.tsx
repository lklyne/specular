import { LayoutGrid } from 'lucide-react'
import type { PanelFileEntityDetail } from '../../../shared/types'
import { fileEntityLabel, mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'
import { PaneField, PaneSection } from './PaneSection'
import { FileDeviceSection } from './FileDeviceSection'
import { FileEntityShell } from './FileEntityShell'

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

function NodeTreeItem({ node, depth }: { node: WireframeNode; depth: number }) {
  const isDark = usePaneTheme()
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
        <NodeTreeItem key={child.id} node={child} depth={depth + 1} />
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

export function WireframeFilePane({ fileEntity }: { fileEntity: PanelFileEntityDetail }) {
  const isDark = usePaneTheme()
  const muted = mutedClass(isDark)
  const label = fileEntityLabel(fileEntity.file).replace(/\.wireframe\.json$/i, '') || 'Wireframe'

  return (
    <FileEntityShell
      icon={<LayoutGrid size={14} className="shrink-0 text-zinc-500" />}
      label={label}
      entityId={fileEntity.id}
    >
      <FileDeviceSection fileEntity={fileEntity} />

      <PaneSection.Root>
        <PaneSection.Label>Theme</PaneSection.Label>
        <div className="flex items-center gap-1.5">
          {THEME_OPTIONS.map((t) => (
            <div key={t.value} className="flex items-center gap-1" title={t.label}>
              <span
                className={`block h-3.5 w-3.5 rounded-full border ${isDark ? 'border-zinc-600' : 'border-zinc-300'}`}
                style={{ background: t.color }}
              />
            </div>
          ))}
          <span className={`ml-1 text-[10px] ${muted}`}>Edit on canvas</span>
        </div>
      </PaneSection.Root>

      <PaneSection.Root>
        <PaneSection.Label>Components</PaneSection.Label>
        <div className="grid grid-cols-2 gap-1">
          {PALETTE_ITEMS.map((item) => (
            <div
              key={item.type}
              className={`flex items-center gap-1.5 rounded px-2 py-1 text-[11px] ${
                isDark
                  ? 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'
                  : 'bg-zinc-100 text-zinc-600 hover:bg-zinc-200'
              }`}
              title="Add via JSON editor"
            >
              <span className="font-mono text-[10px] opacity-60" style={{ width: 12, textAlign: 'center' }}>
                {NODE_TYPE_ICONS[item.type]}
              </span>
              {item.label}
            </div>
          ))}
        </div>
      </PaneSection.Root>

      <PaneField label="Dimensions">
        <div className={`text-[11px] ${muted}`}>
          {fileEntity.width} × {fileEntity.height}
        </div>
      </PaneField>

      <PaneField label="Path">
        <div
          className={`break-all rounded px-2 py-1.5 text-[11px] leading-5 ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}
          title={fileEntity.file}
        >
          {fileEntity.file}
        </div>
      </PaneField>
    </FileEntityShell>
  )
}
