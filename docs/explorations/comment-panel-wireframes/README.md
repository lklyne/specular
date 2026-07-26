# Comment panel wireframes

Design exploration for moving comment threads out of the canvas popup and into
the right panel as a consistent chat interface. Companion to the agent-fix
workflow audit (thread = resumable Claude session stays; the *surface* moves).

Six self-contained HTML docs, each front-loading one decision cluster with
variants and open questions (amber boxes):

| Doc | Decision |
|---|---|
| 01-panel-architecture | Where chat lives in a selection-driven panel: tab bar / takeover / docked split |
| 02-list-thread-navigation | List ↔ thread: drill-in / accordion / docked active thread |
| 03-thread-anatomy | The chat itself: header, context card, run block, diff, resolve suggestion, composer |
| 04-agent-run-states | Run block lifecycle: queued, running, completed, failed, resume-fallback |
| 05-composer-variants | Does sending trigger the agent: always / two actions / toggle chip |
| 06-canvas-choreography | What stays spatial: badges, inline creation, reveal + hover linking |

## View on the canvas

With the Specular app open:

```bash
bash docs/explorations/comment-panel-wireframes/add-to-canvas.sh
```

Places the six docs in a 2-column grid plus an overview sticky. Set
`ORIGIN_X` / `ORIGIN_Y` to control placement. Or open any doc directly in a
browser — they are plain static HTML.

`wireframe.css` is the shared source style; each HTML has it inlined so the
docs stay self-contained when rendered via `local-file://`.
