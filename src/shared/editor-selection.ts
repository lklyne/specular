export interface SelectableTextInput {
  focus: () => void
  select: () => void
}

export function autofocusEditorSelection(
  documentLength: number,
  selectAll: boolean,
): { anchor: number; head: number } {
  return selectAll
    ? { anchor: 0, head: documentLength }
    : { anchor: documentLength, head: documentLength }
}

export function focusAndSelectAll(input: SelectableTextInput): void {
  input.focus()
  input.select()
}
