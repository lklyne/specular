import { useEffect, useState } from "react";

const isEditable = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement &&
  (target.isContentEditable ||
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA");

/**
 * True while the space bar is held — the "hand tool." When active, PageLayer
 * flips each page webview to passthrough so a drag that starts over a live page
 * still pans the host-DOM canvas. This is the native replacement for Specular's
 * aboveView-grabs-input gate: a per-webview flag, not a separate overlay view.
 */
export function usePanTool(): boolean {
  const [active, setActive] = useState(false);

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.code === "Space" && !isEditable(e.target)) {
        e.preventDefault();
        setActive(true);
      }
    };
    const up = (e: KeyboardEvent) => {
      if (e.code === "Space") setActive(false);
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, []);

  return active;
}
