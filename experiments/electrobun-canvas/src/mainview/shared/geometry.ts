// Self-contained geometry primitives. Re-derived (not imported) from the main
// repo's src/shared so the spike has zero coupling to the Electron app.

export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const clamp = (value: number, min: number, max: number): number =>
  Math.max(min, Math.min(max, value));
