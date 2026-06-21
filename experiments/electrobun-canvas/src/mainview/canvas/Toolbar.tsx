interface ToolbarProps {
  panActive: boolean;
  overlayEnabled: boolean;
  onAddSticky: () => void;
  onToggleOverlay: () => void;
}

export function Toolbar({
  panActive,
  overlayEnabled,
  onAddSticky,
  onToggleOverlay,
}: ToolbarProps) {
  return (
    <div className="toolbar">
      <div className="toolbar-title">Electrobun Canvas · layering spike</div>
      <div className="toolbar-actions">
        <button onClick={onAddSticky}>+ Sticky</button>
        <button onClick={onToggleOverlay}>
          {overlayEnabled ? "Hide" : "Show"} passthrough overlay
        </button>
      </div>
      <ul className="toolbar-help">
        <li>
          <b>⌘ + scroll</b> zoom · <b>scroll</b> or <b>space-drag</b> pan
          {panActive ? <em> · panning</em> : null}
        </li>
        <li>
          Drag a <b>sticky</b> into the page overlap, then <b>▲ ▼</b> to restack
        </li>
        <li>A sticky above a page shows through it; below, the page covers it</li>
      </ul>
    </div>
  );
}
