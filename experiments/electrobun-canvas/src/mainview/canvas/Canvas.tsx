import { useEffect, useRef } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { cameraTransform, type Camera } from "../core/camera";
import type { Scene } from "../core/scene";
import { CanvasItemView } from "./CanvasItemView";
import { CursorOverlay } from "./CursorOverlay";

interface CanvasProps {
  camera: Camera;
  scene: Scene;
  selectedId: string | null;
  panActive: boolean;
  overlayEnabled: boolean;
  onWheel: (e: WheelEvent) => void;
  onPanByScreen: (dx: number, dy: number) => void;
  onSelect: (id: string | null) => void;
  onMove: (id: string, dx: number, dy: number) => void;
  onStepZ: (id: string, dir: 1 | -1) => void;
  onEditSticky: (id: string, text: string) => void;
}

export function Canvas(props: CanvasProps) {
  const rootRef = useRef<HTMLDivElement>(null);

  // Wheel must be a non-passive native listener so we can preventDefault the
  // browser's own zoom/scroll. React's onWheel is passive.
  useEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    const handler = (e: WheelEvent) => props.onWheel(e);
    el.addEventListener("wheel", handler, { passive: false });
    return () => el.removeEventListener("wheel", handler);
  }, [props.onWheel]);

  const onPointerDown = (e: ReactPointerEvent) => {
    // Hand tool (space) or middle button → pan the host canvas. Pages are
    // already passthrough while panActive, so this fires even over a page.
    if (props.panActive || e.button === 1) {
      e.preventDefault();
      let prevX = e.clientX;
      let prevY = e.clientY;
      const move = (ev: PointerEvent) => {
        props.onPanByScreen(ev.clientX - prevX, ev.clientY - prevY);
        prevX = ev.clientX;
        prevY = ev.clientY;
      };
      const up = () => {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", up);
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", up);
      return;
    }
    // Empty-canvas click clears selection.
    const target = e.target as HTMLElement;
    if (target === rootRef.current || target.classList.contains("world")) {
      props.onSelect(null);
    }
  };

  return (
    <div
      ref={rootRef}
      className={`canvas${props.panActive ? " panning" : ""}`}
      onPointerDown={onPointerDown}
    >
      <div className="world" style={{ transform: cameraTransform(props.camera) }}>
        {[...props.scene.pages, ...props.scene.stickies].map((entity) => (
          <CanvasItemView
            key={entity.id}
            entity={entity}
            scene={props.scene}
            zoom={props.camera.zoom}
            selectedId={props.selectedId}
            panActive={props.panActive}
            onSelect={props.onSelect}
            onMove={props.onMove}
            onStepZ={props.onStepZ}
            onEditSticky={props.onEditSticky}
          />
        ))}
      </div>
      <CursorOverlay enabled={props.overlayEnabled} />
    </div>
  );
}
