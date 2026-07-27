// Fill for the one primary action in a surface (send a comment, run a fix).
// Reads the surface tokens so light/dark comes from CSS, not an isDark branch.
export const PRIMARY_BUTTON_CLASS =
  'bg-[var(--surface-primary)] text-[var(--surface-primary-foreground)] hover:bg-[var(--surface-primary-hover)]'

// Same fill for buttons that sit in a bordered row and must not show a border.
export const PRIMARY_BUTTON_BORDERED_CLASS = `border-transparent ${PRIMARY_BUTTON_CLASS}`
