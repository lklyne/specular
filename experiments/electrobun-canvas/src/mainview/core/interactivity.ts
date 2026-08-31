// The single rule that governs interactivity for every canvas item, regardless
// of substrate (native webview, host DOM, …). An item accepts internal input
// only when it is the selected item, the hand tool isn't panning over it, AND
// nothing is mid-drag. The drag clause matters because selecting an item flips
// it live on the next render; without it, a native page webview would switch
// from passthrough to live mid-gesture and capture the in-flight drag as a text
// selection. Holding every item inert until pointerup keeps the gesture host-owned.
export const isLive = (
  selected: boolean,
  panActive: boolean,
  dragging: boolean,
): boolean => selected && !panActive && !dragging;
