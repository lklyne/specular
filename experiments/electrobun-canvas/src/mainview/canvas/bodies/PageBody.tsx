import { useEffect, useRef } from "react";
import { pageMaskSelectors, syncMasks } from "../../core/layering";
import type { PageEntity, StickyEntity } from "../../core/scene";
import { EbWebview, type WebviewElement } from "../EbWebview";

interface PageBodyProps {
  page: PageEntity;
  stickies: StickyEntity[];
  live: boolean;
}

// Webview substrate. The inert↔live gate is a single native property:
// passthrough on = click-through (inert), off = native input (live).
export function PageBody({ page, stickies, live }: PageBodyProps) {
  const ref = useRef<WebviewElement>(null);

  // The thesis: punch holes for every item stacked above this page.
  useEffect(() => {
    const el = ref.current;
    if (el) syncMasks(el, pageMaskSelectors(page, stickies));
  }, [page, stickies]);

  useEffect(() => {
    ref.current?.togglePassthrough(!live);
  }, [live]);

  return (
    <EbWebview
      ref={ref}
      className="page-webview"
      src={page.url}
      partition={`persist:${page.id}`}
    />
  );
}
