import { type ReactNode } from 'react'
import { dividerClass, mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'

function Root({ children }: { children: ReactNode }) {
  const isDark = usePaneTheme()
  return (
    <div className={`border-t px-2 pt-2 pb-2 ${dividerClass(isDark)}`}>
      {children}
    </div>
  )
}

function Label({ children }: { children: ReactNode }) {
  const isDark = usePaneTheme()
  return (
    <div className={`mb-1 text-[10px] font-medium ${mutedClass(isDark)}`}>
      {children}
    </div>
  )
}

export const PaneSection = { Root, Label }

export function PaneField({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Root>
      <Label>{label}</Label>
      {children}
    </Root>
  )
}
