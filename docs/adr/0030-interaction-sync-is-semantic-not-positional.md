# Interaction sync is semantic, not positional

Status: Accepted

Sync sets shared navigation and scroll but not interactions: clicking a menu on one page did nothing on its peers, so multi-breakpoint iteration broke the moment a flow left the URL bar. We decided to complete the sync family with **interaction sync** — the source page's hover and clicks replay on every peer — and to make the replay **element-anchored rather than coordinate-based**. The source page's preload captures each event with a descriptive locator bundle (id / testid / role+accessible-name / text / structural path, plus a within-element offset fraction); each peer scores that bundle against its own live DOM (the same scoring vocabulary as `findPresenceTarget`, shared via `src/shared/`) and dispatches trusted input via CDP (`webContents.debugger` `Input.dispatch*`) at *its own* element's position. A **synced cursor** — a presence cursor sourced from the user's mirrored input — renders on each peer, always on: it glides viewport-proportionally when no element is matched and snaps to the peer-resolved element when one is.

## Considered options

- **Coordinate replay** (tee main's existing `forwardPointerToPage` stream, dispatch the same x/y into peers). Rejected: the same coordinates hit different elements whenever breakpoints or layouts differ — which is precisely the multi-breakpoint workflow sync sets exist for. A silent wrong-click is worse than no sync.
- **Untrusted DOM replay** (`dispatchEvent` from the peer preload). Rejected: `isTrusted: false` events skip native behaviors — `:hover`/`:active`, focus side effects, `<select>` popups — so hover-opened menus and real form controls would not track. CDP dispatch is trusted, works on unfocused background views, and reuses the zoom-compensation path the agent driver already exercises. (`sendInputEvent` was rejected for peers because it requires window focus and steals `webContents` focus on mousedown.)
- **Time-window suppression for loop prevention** (like scroll/nav sync). Unnecessary: input authority is already exclusive (ADR 0022) — only one page receives real user input at a time — so capture is enabled only on that page. Replayed events on peers fire DOM listeners but never rebroadcast, structurally.

## Decision details

- **Two-tier honesty: display is best-effort, actions are confident-or-skip.** The synced cursor is always visible while the source page receives input — viewport-proportional gliding as the base layer, element-snap (with target halo) when the locator resolves unambiguously (exact unique identity key, or a clear top score over the runner-up). A click replays only from the anchored state; at a proportional position it is dropped with a cursor wiggle, never dispatched as coordinates — that line is what keeps "semantic, not positional" true for actions while the display is partly positional. The halo is the honesty tell: it appears only where a click would actually land. The wiggle has exactly one meaning: a refused click.
- **Ambient, always on.** Interaction sync is not a sub-toggle; it is what "synced" means. Sync sets are already deliberate (chain-button on a multi-selection) and easy to dissolve. A "follow but don't touch" toggle can be added later with evidence; a matrix of half-synced states cannot easily be removed.
- **Scope.** Hover (`mouseMoved` replay, giving peers real `:hover` states) and clicks in the first cut; text/form input as the next slice. Raw keyboard shortcuts and drag gestures are explicit non-goals — drags are coordinate-bound with no stable mid-gesture target and degrade to today's behavior (source works, peers don't follow). Peers whose current URL is a different origin are skipped entirely.
- **Every shared navigation is a fresh sync point.** Divergence is inherent to best-effort mirroring (a skipped event forks peer UI state), so interaction sync is only best-effort *between* navigations. Nav sync already propagates both reloads and route changes across the set, and each peer loads the fresh URL from scratch, wiping forked UI state back to a common baseline — recovery needs no new UI or machinery.

## Consequences

- The locator bundle becomes a wire format shared by the synced cursor, click replay, and (later) text-input sync — changing its shape after text-input lands touches capture, fan-out, and every resolver.
- Every page in an active sync set may get a debugger attached on first replayed event; rare sites behave differently under DevTools attachment.
- A mirrored click's side effects (navigation, scroll) cascade into nav/scroll sync on the source's peers; suppression there already arbitrates, but "one click on a link in a 3-page set produces exactly one navigation per peer" is a required integration scenario.
