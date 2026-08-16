# Specular

Part web browser and part canvas, specular is a hybrid design tool for thinking through ideas spatially and iterating on software.

Code is the source of truth for making ideas real, but long chat threads and single browser tabs aren't great for the exploratory, divergent thinking that designing something great requires. A canvas fits that work better — but there's usually friction moving back and forth between canvas and code. Specular sidesteps it with full-featured browser tabs on a canvas: the actual website, product, or prototype lives spatially alongside notes, images, and drawings, designed so people and agents can vibe out designs and research in one place.

<!-- TODO: Add screenshot or GIF demo here -->

## What you can do

- Explore different directions and see them side-by-side
- Annotate live websites and pass the feedback straight back to an agent
- Lay out a page at multiple device breakpoints to check responsiveness
- Ask an agent to share its thinking visually, or ingest a repo's design system
- Switch to browser mode for a classic tab-based browser

## Key features

- **Canvas** — Real browser windows on an infinite, zoomable canvas
- **Agent-friendly** — Agents drive the canvas through a `specular` CLI (primary) or MCP server (fallback): creating frames, navigating, inspecting the DOM, clicking, typing, screenshotting
- **Agent presence** — See an agent's live cursor and task status as it works alongside you
- **Annotations** — Comment on any frame, usable by people and agents
- **Device frames** — Preview sites at preset device sizes with visual device shells
- **Groups & edges** — Organize frames into freeform/row/grid groups and draw connections between them
- **Local-first** — No sign-in, no account; full undo history backed by Yjs CRDTs
- **Open formats** — Layout uses [JSON Canvas](https://jsoncanvas.org); text and media are plain `.md`, `.png`, and `.webm` files on disk

## Inspiration and related products
- [Paper](https://paper.design): imo the best full-featured design tool with agent collaboration
- [Agentation](https://agentation.com): inspiration for visual edits and commenting
- [Polypane](https://polypane.app): for viewing a webpage across multiple breakpoints in one place
- [Obsidian](https://obsidian.md): inspiration from local storage format and its lightweight canvas
- [agent-browser](https://agent-browser.dev): inspiration for cli based web automation. Wrapped in specular skill for browser automation.

## System requirements

- macOS 12+ (Apple Silicon or Intel)
- Windows and Linux builds aren't currently planned

## Installation

Download the latest release from the [GitHub Releases](https://github.com/lklyne/specular/releases) page.

Updates are delivered automatically via `update-electron-app`. You'll be prompted to restart when a new version is ready.

## Using with AI agents

The `specular` CLI is the main interface for agents — composable commands that fit an agent's working loop:

```bash
specular canvas                          # inspect the current canvas
specular add page <url>                  # pull a live page onto the canvas
specular snapshot -i                     # get element refs for the selected frame
specular annotate "<feedback>"           # leave a comment for a human or agent
```

A Claude Code skill ships with the app — see [`resources/skills/specular/SKILL.md`](resources/skills/specular/SKILL.md) for the full surface. An [MCP server](src/main/mcp-tools.ts) covers the same operations for clients that prefer it.

## Security

To report a security vulnerability, see [SECURITY.md](SECURITY.md).

## License

Licensed under [PolyForm Shield 1.0.0](LICENSE.md). Copyright © 2026 Lyle Klyne.
