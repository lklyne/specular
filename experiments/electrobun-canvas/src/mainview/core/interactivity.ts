// The single rule that governs interactivity for every canvas item, regardless
// of substrate (native webview, host DOM, …). An item accepts internal input
// only when it is the selected item AND the hand tool isn't panning over it.
export const isLive = (selected: boolean, panActive: boolean): boolean =>
  selected && !panActive;
