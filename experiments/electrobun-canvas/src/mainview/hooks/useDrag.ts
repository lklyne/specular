import type { PointerEvent as ReactPointerEvent } from "react";

/**
 * Begin a left-button pointer drag, reporting deltas already converted from
 * screen pixels into world units (so callers mutate canvas coordinates
 * directly). Window-level listeners keep the drag alive even if the pointer
 * leaves the element. Intended to be called from an `onPointerDown` handler.
 */
export function startPointerDrag(
  e: ReactPointerEvent,
  zoom: number,
  onDrag: (dxWorld: number, dyWorld: number) => void,
  onEnd?: () => void,
): void {
  if (e.button !== 0) return;
  e.stopPropagation();
  let prevX = e.clientX;
  let prevY = e.clientY;

  const move = (ev: PointerEvent) => {
    onDrag((ev.clientX - prevX) / zoom, (ev.clientY - prevY) / zoom);
    prevX = ev.clientX;
    prevY = ev.clientY;
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    onEnd?.();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}
