import { Copy, Trash2 } from 'lucide-react'
import { type ReactNode } from 'react'
import { paneActionBtnClass, paneDeleteBtnClass } from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { usePaneTheme } from '../PaneContext'
import { PaneHeader } from './PaneHeader'

export function FileEntityShell({
  icon,
  label,
  entityId,
  children,
}: {
  icon: ReactNode
  label: string
  entityId: string
  children?: ReactNode
}) {
  const isDark = usePaneTheme()
  return (
    <div className="flex flex-col">
      <PaneHeader
        icon={icon}
        label={label}
        actions={
          <>
            <button
              type="button"
              className={paneActionBtnClass(isDark)}
              onClick={() => rightDetailsPanelApi.duplicateFileEntity(entityId)}
              title="Duplicate"
            >
              <Copy size={14} />
            </button>
            <button
              type="button"
              className={paneDeleteBtnClass(isDark)}
              onClick={() => rightDetailsPanelApi.deleteFileEntity(entityId)}
              title="Delete"
            >
              <Trash2 size={14} />
            </button>
          </>
        }
      />
      {children}
    </div>
  )
}
