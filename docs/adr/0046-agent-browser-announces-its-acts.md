# ADR 0046: agent-browser announces its acts and waits for the cursor

**Status:** Proposed. Built this way in `native/`. The user has not reviewed this ADR.
**Date:** 2026-10-10
**Related:** [ADR 0029](./0029-presence-acts-anchored-to-truth.md), whose rule this keeps and whose machinery it replaces in the Rust app. [ADR 0042](./0042-update-view-render-loop-effects-only-io.md), the loop the cursor runs in.

## Context

ADR 0029 says an agent's cursor arrives before the page reacts. In Electron the app cannot ask agent-browser where it will act, so it works that out from the outside. The CDP proxy reads `DOM.getBoxModel` answers to guess the target. The CLI sends an intent before it spawns agent-browser, and main resolves the selector a second time to give the cursor a head start. A timer of up to 300 ms holds the input event, because main cannot see when the renderer's cursor has landed. `@eN` refs get no head start at all, since the ref table lives in agent-browser's daemon.

Two things are different in the Rust app. Its CDP proxy runs in the app's own process, and the cursor is drawn by the same loop that would release the act. agent-browser is also an Apache-2.0 Rust program, so it can be changed.

Moving the verbs into the app was considered first and turned down. A `click @e5` owned by the app needs the snapshot that made `@e5`, and then every verb that takes a ref, which is agent-browser's whole 690 KB dispatcher.

## Decision

agent-browser says where it is about to act and waits to be told to go.

- A patch to agent-browser (`native/patches/agent-browser-will-act.patch`, built by `native/scripts/build-agent-browser.sh` at the tag the Electron app pins) adds one call before each targeted verb. It resolves the element's centre and sends `Specular.willAct` with the verb, the point in viewport CSS pixels and the daemon's session name, on the browser level of the page's CDP socket.
- `PageProxy` answers that method itself and holds the answer (`Out::Act`). The shell turns it into `Event::AgentAct`. `specular-interact/src/presence.rs` starts the cursor of that agent toward the point and returns `Effect::AgentGo` on the tick the cursor is drawn there. The shell then sends the held answer.
- agent-browser resolves the element again after the answer and dispatches. A page that moved during the travel is still hit where the element now is.
- The travel takes 90 ms plus 0.35 ms a screen pixel, at most 320 ms, then rests 40 ms. A cursor already on the point, a page that is not on the canvas shown, and a shell with transitions off are answered at once.
- A read of one element (`get text`, `is visible`, `highlight`, `scrollintoview`) is announced with `wait: false`. The proxy answers it at once and the cursor goes there anyway, so nothing is held for a read and the act that follows on the same element starts with the cursor on it.
- An act on a point a cursor is already travelling to joins that travel. It waits for what is left of it and no new travel starts.
- A drag is two held announcements. `drag` at the source comes before the press and `drop` at the destination comes after it, so the page sees the button held while the cursor crosses.
- `press`, `keyboard type` and `keyboard inserttext` announce the centre of the focused element, and nothing when only the body has focus.
- Each announcement carries the page's scroll offset. The shell gives it to the app as the page's scroll before the act, since the page's own notice of a scroll made a moment earlier may not have come. The cursor keeps its target in document pixels, follows the page as it scrolls, and is not drawn once its target has left the viewport.
- A cursor belongs to an agent, which is the session name up to its last colon, so one cursor crosses the canvas as its agent goes from page to page. It leaves 10 s after the agent's last act.

The patched binary gives up on a browser that does not know the method. Plain Chrome answers with an error, and the hook is not sent again by that daemon. An answer that takes more than 3 s is treated as a go.

## Consequences

- The intent route, the second resolve, the box-model reading and the timer are not ported. `/session/presence/intent` stays accepted and dropped.
- Every target is treated alike, refs included.
- The CLI is unchanged. It runs whatever `AGENT_BROWSER_PATH` names, and the stock binary still works against the Rust app with no cursor.
- We carry a patch. It is 730 lines, most of them one-line calls, and it has to be applied again at each agent-browser bump.
- `eval`, `mouse`, `wheel`, `scroll`, the clipboard keys and `uploadfile` are not announced. The cursor stays where it was, as ADR 0029 rule 4 asks.
- The CLI's own `scrollintoview` before each mutation is not needed here. agent-browser scrolls the target into view when it resolves it. The step is left in the CLI, which the Electron app shares and where it was not tested.

## Measured

On a debug build with two 800 by 500 pages and the CLI driving them:

- A click with the stock binary took 175 ms from the shell. With the patch it took 320 to 390 ms, of which 150 to 230 ms was the cursor's travel and rest.
- The page's `mousedown` fired 3 ms after the app sent the go.
- `fill`, `select` and a click by ref each moved the cursor and landed.
- A click straight after a read of the same element took 183 ms, the same as the stock binary. The same click with the cursor elsewhere took 373 ms.
- A click sent straight after the read, with the cursor still on its way, waited 80 to 100 ms for it in two runs of three and 218 ms in the first. A click with no read before it waited 300 to 320 ms over the same distance.
- A drag held the button for 437 ms between `mousedown` at the source and `mouseup` at the destination.
- With the CLI's `scrollintoview` step taken out of a copy of the CLI, a click and a fill on elements 1800 px down the page still landed, with the stock binary and the patched one. It saved no time that could be measured: about 180 ms either way.
- A cursor on a button stayed on it when the page scrolled 150 px and was not drawn after 600 px more.

Not run: an iframe target, the 3 s timeout, two agents on one page, and a release build.
