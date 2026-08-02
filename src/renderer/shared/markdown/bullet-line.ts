/**
 * The one definition of "this line is a bullet item": optional indent
 * (captured), a `-`/`*`/`+` marker, one space. Commands, keymaps, format
 * state, and whole-note toggles all key off this — keep them agreeing.
 */
export const BULLET_LINE = /^(\s*)[-*+] /
