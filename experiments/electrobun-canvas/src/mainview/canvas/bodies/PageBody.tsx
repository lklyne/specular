import { useEffect, useRef } from "react";
import { pageMaskSelectors, syncMasks } from "../../core/layering";
import type { Entity, PageEntity, StickyEntity } from "../../core/scene";
import { EbWebview, type WebviewElement } from "../EbWebview";

interface PageBodyProps {
  page: PageEntity;
  stickies: StickyEntity[];
  selected: Entity | null;
  live: boolean;
  zoom: number;
}

// Match canvas zoom to WebKit page zoom. The native overlay's rect already
// tracks the host card's *scaled* bounding box (overlaySync reads
// getBoundingClientRect, which includes the world's CSS scale). So at zoom z the
// page's layout viewport is `frame.width * z` CSS px — the page reflows wider as
// you zoom in (text stays the same size) instead of magnifying, and it stops
// laying out at its design width. Setting CSS `zoom: z` on the document cancels
// that out: the page lays out at `frame.width` logical px again (responsive to
// the chrome; media queries see the design width) and is then magnified z×,
// which is exactly Electron's `webContents.setZoomFactor`.
//
// We go through executeJavascript because the <electrobun-webview> tag exposes
// no native page-zoom RPC (the FFI `webviewSetPageZoom` is reachable from a bun
// BrowserView, but not from a host-DOM tag without forking the runtime).
const applyPageZoom = (el: WebviewElement, zoom: number) => {
  el.executeJavascript(
    `document.documentElement.style.zoom = ${JSON.stringify(String(zoom))}`,
  );
};

// Webview substrate. The inert↔live gate is a single native property:
// passthrough on = click-through (inert), off = native input (live).
export function PageBody({ page, stickies, selected, live, zoom }: PageBodyProps) {
  const ref = useRef<WebviewElement>(null);

  // The thesis: punch holes for every item stacked above this page.
  //
  // Electrobun only re-reads mask geometry when the page webview's OWN rect
  // changes (overlaySync's sync() early-returns otherwise). A sticky moving
  // above us doesn't move the page, so the hole would trail behind it. This
  // effect re-runs on every sticky mutation (new `stickies` array per move), so
  // forcing a sync here re-reads the mask elements' positions each frame.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    syncMasks(el, pageMaskSelectors(page, stickies, selected));
    el.syncDimensions(true);
  }, [page, stickies, selected]);

  useEffect(() => {
    ref.current?.togglePassthrough(!live);
  }, [live]);

  // Keep page zoom locked to canvas zoom. The effect covers live zoom gestures;
  // `dom-ready` re-applies after every navigation (each load resets the inline
  // style). Force a dimension sync too so the native rect re-tracks the
  // freshly-scaled card in the same frame rather than trailing it.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    applyPageZoom(el, zoom);
    el.syncDimensions(true);
    const onReady = () => applyPageZoom(el, zoom);
    el.on("dom-ready", onReady);
    return () => el.off("dom-ready", onReady);
  }, [zoom]);

  return (
    <EbWebview
      ref={ref}
      className="page-webview"
      src={page.url}
      partition={`persist:${page.id}`}
    />
  );
}
