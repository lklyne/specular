import type { Entity, Scene } from "../core/scene";
import { CanvasItem } from "./CanvasItem";
import { PageBody } from "./bodies/PageBody";
import { StickyBody } from "./bodies/StickyBody";

interface CanvasItemViewProps {
  entity: Entity;
  scene: Scene;
  zoom: number;
  selectedId: string | null;
  panActive: boolean;
  dragging: boolean;
  onSelect: (id: string) => void;
  onDragChange: (active: boolean) => void;
  onMove: (id: string, dx: number, dy: number) => void;
  onResize: (id: string, dw: number, dh: number) => void;
  onStepZ: (id: string, dir: 1 | -1) => void;
  onEditSticky: (id: string, text: string) => void;
}

const hostOf = (url: string): string => {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
};

// The single dispatch point — the spike's analog of the main app's
// RendererSwitch. Each new kind adds one case here plus a body adapter; the
// shell, selection rule, drag, restack, outline, and z-order are untouched.
export function CanvasItemView({
  entity,
  scene,
  zoom,
  selectedId,
  panActive,
  dragging,
  onSelect,
  onDragChange,
  onMove,
  onResize,
  onStepZ,
  onEditSticky,
}: CanvasItemViewProps) {
  return (
    <CanvasItem
      id={entity.id}
      frame={entity.frame}
      z={entity.z}
      title={entity.kind === "page" ? hostOf(entity.url) : "note"}
      zoom={zoom}
      selected={selectedId === entity.id}
      panActive={panActive}
      dragging={dragging}
      onSelect={onSelect}
      onDragChange={onDragChange}
      onMove={onMove}
      onResize={onResize}
      onStepZ={onStepZ}
    >
      {(live) =>
        entity.kind === "page" ? (
          <PageBody
            page={entity}
            stickies={scene.stickies}
            selected={
              [...scene.pages, ...scene.stickies].find(
                (e) => e.id === selectedId,
              ) ?? null
            }
            live={live}
            zoom={zoom}
          />
        ) : (
          <StickyBody sticky={entity} live={live} onEdit={onEditSticky} />
        )
      }
    </CanvasItem>
  );
}
