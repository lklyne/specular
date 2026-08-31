import type { StickyEntity } from "../../core/scene";

interface StickyBodyProps {
  sticky: StickyEntity;
  live: boolean;
  onEdit: (id: string, text: string) => void;
}

// Host-DOM substrate. The inert↔live gate is `contentEditable`: text is editable
// only while live. When live, swallow the pointerdown so the shell's drag/select
// doesn't fight the caret (the chrome remains the move handle).
export function StickyBody({ sticky, live, onEdit }: StickyBodyProps) {
  return (
    <div className="sticky-surface" style={{ background: sticky.color }}>
      <div
        className="sticky-text"
        contentEditable={live}
        suppressContentEditableWarning
        onPointerDown={(e) => {
          if (live) e.stopPropagation();
        }}
        onBlur={(e) => onEdit(sticky.id, e.currentTarget.textContent ?? "")}
      >
        {sticky.text}
      </div>
    </div>
  );
}
