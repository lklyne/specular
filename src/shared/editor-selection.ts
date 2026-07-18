export interface SelectableTextInput {
  focus: () => void
  select: () => void
}

export function autofocusEditorSelection(
  documentLength: number,
): { anchor: number; head: number } {
  return { anchor: 0, head: documentLength }
}

export function focusAndSelectAll(input: SelectableTextInput): void {
  input.focus()
  input.select()
}
