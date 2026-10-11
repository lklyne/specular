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
- A cursor belongs to an agent, which is the session name up to its last colon, so one cursor crosses the canvas as its agent goes from page to page. It leaves 10 s after the agent's last act.

The patched binary gives up on a browser that does not know the method. Plain Chrome answers with an error, and the hook is not sent again by that daemon. An answer that takes more than 3 s is treated as a go.

## Consequences

- The intent route, the second resolve, the box-model reading and the timer are not ported. `/session/presence/intent` stays accepted and dropped.
- Every target is treated alike, refs included.
- The CLI is unchanged. It runs whatever `AGENT_BROWSER_PATH` names, and the stock binary still works against the Rust app with no cursor.
- We carry a patch. It is 320 lines, most of them one-line calls, and it has to be applied again at each agent-browser bump.
- `drag`, `press`, `eval` and the other verbs with no single target are not announced. The cursor stays where it was, as ADR 0029 rule 4 asks.

## Measured

On a debug build with two 800 by 500 pages and the CLI driving them:

- A click with the stock binary took 175 ms from the shell. With the patch it took 320 to 390 ms, of which 150 to 230 ms was the cursor's travel and rest.
- The page's `mousedown` fired 3 ms after the app sent the go.
- `fill`, `select` and a click by ref each moved the cursor and landed.

Not run: an iframe target, the 3 s timeout, two agents on one page, and a release build.
