(function(){// src/bun/preload/events.ts
function emitWebviewEvent(eventName, detail) {
  setTimeout(() => {
    const bridge = window.__electrobunEventBridge || window.__electrobunInternalBridge;
    bridge?.postMessage(JSON.stringify({
      id: "webviewEvent",
      type: "message",
      payload: {
        id: window.__electrobunWebviewId,
        eventName,
        detail
      }
    }));
  });
}
function initLifecycleEvents() {
  window.addEventListener("load", () => {
    if (window === window.top) {
      emitWebviewEvent("dom-ready", document.location.href);
    }
  });
  window.addEventListener("popstate", () => {
    emitWebviewEvent("did-navigate-in-page", window.location.href);
  });
  window.addEventListener("hashchange", () => {
    emitWebviewEvent("did-navigate-in-page", window.location.href);
  });
}
var cmdKeyHeld = false;
var cmdKeyTimestamp = 0;
var CMD_KEY_THRESHOLD_MS = 500;
function isCmdHeld() {
  if (cmdKeyHeld)
    return true;
  return Date.now() - cmdKeyTimestamp < CMD_KEY_THRESHOLD_MS && cmdKeyTimestamp > 0;
}
function initCmdClickHandling() {
  window.addEventListener("keydown", (event) => {
    if (event.key === "Meta" || event.metaKey) {
      cmdKeyHeld = true;
      cmdKeyTimestamp = Date.now();
    }
  }, true);
  window.addEventListener("keyup", (event) => {
    if (event.key === "Meta") {
      cmdKeyHeld = false;
      cmdKeyTimestamp = Date.now();
    }
  }, true);
  window.addEventListener("blur", () => {
    cmdKeyHeld = false;
  });
  window.addEventListener("click", (event) => {
    if (event.metaKey || event.ctrlKey) {
      const anchor = event.target?.closest?.("a");
      if (anchor && anchor.href) {
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation();
        emitWebviewEvent("new-window-open", JSON.stringify({
          url: anchor.href,
          isCmdClick: true,
          isSPANavigation: false
        }));
      }
    }
  }, true);
}
function initSPANavigationInterception() {
  const originalPushState = history.pushState;
  const originalReplaceState = history.replaceState;
  history.pushState = function(state, title, url) {
    if (isCmdHeld() && url) {
      const resolvedUrl = new URL(String(url), window.location.href).href;
      emitWebviewEvent("new-window-open", JSON.stringify({
        url: resolvedUrl,
        isCmdClick: true,
        isSPANavigation: true
      }));
      return;
    }
    return originalPushState.apply(this, [state, title, url]);
  };
  history.replaceState = function(state, title, url) {
    if (isCmdHeld() && url) {
      const resolvedUrl = new URL(String(url), window.location.href).href;
      emitWebviewEvent("new-window-open", JSON.stringify({
        url: resolvedUrl,
        isCmdClick: true,
        isSPANavigation: true
      }));
      return;
    }
    return originalReplaceState.apply(this, [state, title, url]);
  };
}
function initOverscrollPrevention() {
  document.addEventListener("DOMContentLoaded", () => {
    const style = document.createElement("style");
    style.type = "text/css";
    style.appendChild(document.createTextNode("html, body { overscroll-behavior: none; }"));
    document.head.appendChild(style);
  });
}

// src/bun/preload/index-sandboxed.ts
initLifecycleEvents();
initCmdClickHandling();
initSPANavigationInterception();
initOverscrollPrevention();
})();