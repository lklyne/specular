import { useEffect, useRef } from "react";
import { pageMaskSelectors, syncMasks } from "../core/layering";
import type { PageEntity, StickyEntity } from "../core/scene";
import { startPointerDrag } from "../hooks/useDrag";
import { EbWebview, type WebviewElement } from "./EbWebview";

interface PageLayerProps {
  page: PageEntity;
  stickies: StickyEntity[];
  zoom: number;
  panActive: boolean;
  selected: boolean;
  onSelect: (id: string) => void;
  onMove: (id: string, dx: number, dy: number) => void;
  onStepZ: (id: string, dir: 1 | -1) => void;
}

const hostOf = (url: string): string => {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
};

export function PageLayer({
  page,
  stickies,
  zoom,
  panActive,
  selected,
  onSelect,
  onMove,
  onStepZ,
}: PageLayerProps) {
  const ref = useRef<WebviewElement>(null);

  // The whole thesis in two lines: recompute which host-DOM elements should
  // punch through this page (its own chrome + every sticky stacked above it),
  // and reconcile the webview's live mask set whenever the z-order changes.
  useEffect(() => {
    const el = ref.current;
    if (el) syncMasks(el, pageMaskSelectors(page, stickies));
  }, [page, stickies]);

  // Hand tool: make the page click-through so the host canvas can pan beneath it.
  useEffect(() => {
    ref.current?.togglePassthrough(panActive);
  }, [panActive]);

  return (
    <div
      className={`page${selected ? " selected" : ""}`}
      style={{
        left: page.frame.x,
        top: page.frame.y,
        width: page.frame.width,
        height: page.frame.height,
      }}
      data-page-id={page.id}
    >
      <EbWebview
        ref={ref}
        className="page-webview"
        src={page.url}
        partition={`persist:${page.id}`}
      />
      {/* Host-DOM chrome that floats over the live page. It only renders above
          the native webview because it is in this page's mask set (see
          pageChromeSelector); the punched hole is also what lets it receive the
          drag pointerdown. */}
      <div
        className="page-chrome"
        data-page-chrome={page.id}
        onPointerDown={(e) => {
          onSelect(page.id);
          startPointerDrag(e, zoom, (dx, dy) => onMove(page.id, dx, dy));
        }}
      >
        <span className="page-title">{hostOf(page.url)}</span>
        <span className="z-controls">
          <button onPointerDown={(e) => e.stopPropagation()} onClick={() => onStepZ(page.id, 1)}>
            ▲
          </button>
          <button onPointerDown={(e) => e.stopPropagation()} onClick={() => onStepZ(page.id, -1)}>
            ▼
          </button>
        </span>
      </div>
    </div>
  );
}
