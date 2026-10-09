import { useCallback, useState } from "react";
import { IDENTITY, panBy, zoomAt, type Camera } from "../core/camera";

// Wheel classification mirrors the main repo's classifyViewportWheel: modifier
// (⌘/Ctrl) → zoom toward cursor, otherwise two-axis pan. The improvement over
// Specular: camera state lives entirely in the renderer, so a pan/zoom is a
// local setState — no IPC round-trip to the main process per gesture frame.
const ZOOM_SENSITIVITY = 0.0015;

export function useCamera() {
  const [camera, setCamera] = useState<Camera>(IDENTITY);

  const onWheel = useCallback((e: WheelEvent) => {
    e.preventDefault();
    if (e.metaKey || e.ctrlKey) {
      const factor = Math.exp(-e.deltaY * ZOOM_SENSITIVITY);
      setCamera((c) => zoomAt(c, factor, e.clientX, e.clientY));
    } else {
      setCamera((c) => panBy(c, -e.deltaX, -e.deltaY));
    }
  }, []);

  const panByScreen = useCallback((dx: number, dy: number) => {
    setCamera((c) => panBy(c, dx, dy));
  }, []);

  return { camera, onWheel, panByScreen };
}
