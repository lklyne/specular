import type { PointerEvent as ReactPointerEvent, ReactNode } from "react";
import type { Rect } from "../shared/geometry";
import { isLive } from "../core/interactivity";
import { startPointerDrag } from "../hooks/useDrag";

// The shared shell for EVERY canvas item, whatever its substrate. It owns
// everything that is identical across kinds — placement, host-side stacking,
// the select hit-target, the selection outline, and the chrome (title + drag
// handle + restack). The only thing that varies per kind is the body, supplied
// as a render-prop that receives the derived `live` flag (see isLive).
//
// This is the repo's compound-chrome pattern (EntityChrome.Root/.DragTrigger/
// .Title/.Actions): style once here, compose a different body per use.

interface CanvasItemProps {
  id: string;
  frame: Rect;
  z: number;
  title: string;
  zoom: number;
  selected: boolean;
  panActive: boolean;
  dragging: boolean;
  onSelect: (id: string) => void;
  onDragChange: (active: boolean) => void;
  onMove: (id: string, dx: number, dy: number) => void;
  onResize: (id: string, dw: number, dh: number) => void;
  onStepZ: (id: string, dir: 1 | -1) => void;
  children: (live: boolean) => ReactNode;
}

export function CanvasItem({
  id,
  frame,
  z,
  title,
  zoom,
  selected,
  panActive,
  dragging,
  onSelect,
  onDragChange,
  onMove,
  onResize,
  onStepZ,
  children,
}: CanvasItemProps) {
  const live = isLive(selected, panActive, dragging);

  const beginDrag = (e: ReactPointerEvent) => {
    onDragChange(true);
    startPointerDrag(
      e,
      zoom,
      (dx, dy) => onMove(id, dx, dy),
      () => onDragChange(false),
    );
  };

  const selectAndDrag = (e: ReactPointerEvent) => {
    onSelect(id);
    beginDrag(e);
  };

  return (
    <div
      className={`item${selected ? " selected" : ""}${live ? " live" : ""}`}
      style={{
        left: frame.x,
        top: frame.y,
        width: frame.width,
        height: frame.height,
        zIndex: z,
      }}
      data-item-id={id}
      onPointerDown={(e) => {
        if (panActive) return; // let the event bubble to the pan handler
        onSelect(id);
        // While inert, a body drag moves the item. While live, the body owns
        // its own input (page scroll, text caret) and only the chrome moves it.
        if (!live) beginDrag(e);
      }}
    >
      <div
        className="item-chrome"
        onPointerDown={(e) => {
          if (panActive) return;
          selectAndDrag(e);
        }}
      >
        <span className="item-title">{title}</span>
        {!live && <span className="item-hint">click to interact</span>}
        <span className="z-controls">
          <button onPointerDown={(e) => e.stopPropagation()} onClick={() => onStepZ(id, 1)}>
            ▲
          </button>
          <button onPointerDown={(e) => e.stopPropagation()} onClick={() => onStepZ(id, -1)}>
            ▼
          </button>
        </span>
      </div>
      <div className="item-body">{children(live)}</div>
      {/* Selection chrome: real elements (not a CSS outline) so each has its own
          rect that pages can punch a mask hole for — see pageMaskSelectors.
          Resizing a page changes its frame, which IS its viewport (breakpoints). */}
      {selected && (
        <>
          <div className="selection-ring" />
          <div
            className="resize-handle"
            onPointerDown={(e) => {
              if (panActive) return;
              onDragChange(true);
              startPointerDrag(
                e,
                zoom,
                (dx, dy) => onResize(id, dx, dy),
                () => onDragChange(false),
              );
            }}
          />
        </>
      )}
    </div>
  );
}
