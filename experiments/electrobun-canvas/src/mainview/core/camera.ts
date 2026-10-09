import { clamp, type Point } from "../shared/geometry";

// A camera is a pan offset (in screen pixels) plus a zoom scale. World→screen
// is `screen = world * zoom + pan`. This is the same transform the main repo
// applies in src/shared/coords.ts, but re-derived as a small immutable value
// type with pure helpers — dropping the LayoutUpdateData / canvasOrigin
// baggage that only exists to serialize the layout cache across IPC.

export const MIN_ZOOM = 0.02;
export const MAX_ZOOM = 3.0;

export interface Camera {
  /** Pan offset, screen pixels. */
  x: number;
  y: number;
  zoom: number;
}

export const IDENTITY: Camera = { x: 0, y: 0, zoom: 1 };

export const clampZoom = (zoom: number): number => clamp(zoom, MIN_ZOOM, MAX_ZOOM);

export const screenToWorld = (cam: Camera, p: Point): Point => ({
  x: (p.x - cam.x) / cam.zoom,
  y: (p.y - cam.y) / cam.zoom,
});

/** Translate the view by a screen-space delta (e.g. a pan drag). */
export const panBy = (cam: Camera, dx: number, dy: number): Camera => ({
  ...cam,
  x: cam.x + dx,
  y: cam.y + dy,
});

/**
 * Zoom by `factor` while keeping the world point under (screenX, screenY)
 * fixed on screen — the standard "zoom toward the cursor" behavior.
 */
export const zoomAt = (
  cam: Camera,
  factor: number,
  screenX: number,
  screenY: number,
): Camera => {
  const zoom = clampZoom(cam.zoom * factor);
  const world = screenToWorld(cam, { x: screenX, y: screenY });
  return {
    zoom,
    x: screenX - world.x * zoom,
    y: screenY - world.y * zoom,
  };
};

/** CSS transform for the world container. */
export const cameraTransform = (cam: Camera): string =>
  `translate(${cam.x}px, ${cam.y}px) scale(${cam.zoom})`;
