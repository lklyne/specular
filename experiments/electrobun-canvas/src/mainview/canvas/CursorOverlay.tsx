import { useEffect, useRef } from "react";
import { EbWebview, type WebviewElement } from "./EbWebview";

// Output-only overlay: a transparent, passthrough webview stretched over the
// whole window. It paints on top of every page yet swallows no input — proving
// the native equivalent of Specular's cursorOverlayWindow hack is a one-liner
// here (transparent + passthrough) rather than a sibling BrowserWindow.
//
// Note: because it is passthrough, it cannot itself observe the mouse, so the
// content is a static watermark. The point being demonstrated is that you can
// still click and scroll the pages *through* it.
const OVERLAY_HTML = `<!doctype html><html><head><meta charset="utf-8"><style>
  html,body{margin:0;height:100%;background:transparent;overflow:hidden;
    font-family:-apple-system,BlinkMacSystemFont,sans-serif;user-select:none}
  .badge{position:fixed;top:14px;left:50%;transform:translateX(-50%);
    background:rgba(20,20,30,.78);color:#fff;padding:7px 14px;border-radius:999px;
    font-size:12px;letter-spacing:.2px;white-space:nowrap}
  .mark{position:fixed;inset:0;display:flex;align-items:center;justify-content:center;
    color:rgba(60,90,200,.10);font-size:120px;font-weight:800;transform:rotate(-20deg)}
</style></head><body>
  <div class="mark">PASSTHROUGH</div>
  <div class="badge">overlay webview on top — clicks &amp; scroll fall through to the pages</div>
</body></html>`;

export function CursorOverlay({ enabled }: { enabled: boolean }) {
  const ref = useRef<WebviewElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.toggleTransparent(true);
    el.togglePassthrough(true);
  }, [enabled]);

  if (!enabled) return null;
  return <EbWebview ref={ref} className="cursor-overlay" html={OVERLAY_HTML} />;
}
