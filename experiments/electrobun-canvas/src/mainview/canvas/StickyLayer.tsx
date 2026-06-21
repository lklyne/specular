import type { StickyEntity } from "../core/scene";
import { startPointerDrag } from "../hooks/useDrag";

interface StickyLayerProps {
  sticky: StickyEntity;
  zoom: number;
  selected: boolean;
  onSelect: (id: string) => void;
  onMove: (id: string, dx: number, dy: number) => void;
  onStepZ: (id: string, dir: 1 | -1) => void;
  onEdit: (id: string, text: string) => void;
}

// A sticky is plain host DOM. Whether it appears in front of or behind any given
// page is decided entirely by that page's mask set (driven by the shared z),
// never by this element's own z-index.
export function StickyLayer({
  sticky,
  zoom,
  selected,
  onSelect,
  onMove,
  onStepZ,
  onEdit,
}: StickyLayerProps) {
  return (
    <div
      className={`sticky${selected ? " selected" : ""}`}
      style={{
        left: sticky.frame.x,
        top: sticky.frame.y,
        width: sticky.frame.width,
        height: sticky.frame.height,
        background: sticky.color,
      }}
      data-sticky-id={sticky.id}
      onPointerDown={(e) => {
        if ((e.target as HTMLElement).isContentEditable) return;
        onSelect(sticky.id);
        startPointerDrag(e, zoom, (dx, dy) => onMove(sticky.id, dx, dy));
      }}
    >
      <div
        className="sticky-text"
        contentEditable
        suppressContentEditableWarning
        onBlur={(e) => onEdit(sticky.id, e.currentTarget.textContent ?? "")}
      >
        {sticky.text}
      </div>
      {selected && (
        <div className="z-controls sticky-z">
          <button onPointerDown={(e) => e.stopPropagation()} onClick={() => onStepZ(sticky.id, 1)}>
            ▲
          </button>
          <button onPointerDown={(e) => e.stopPropagation()} onClick={() => onStepZ(sticky.id, -1)}>
            ▼
          </button>
        </div>
      )}
    </div>
  );
}
