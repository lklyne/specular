# Specular

Part web browser and part canvas, specular is a hybrid design tool for thinking through ideas spatially and iterating on software. 

Code is the source of truth for making ideas real, but long chat threads and single browser tabs aren't ideal for the exploratory, divergent, and visual thinking that's often required to design something great. A canvas is a much more familiar place for this type of work, but there's a gap between what you see on the canvas and what is built. There's lots of products and tools that bridge this gap in a variety of ways, but there's always some level of friction moving back and forth between canvas and code.

Specular sidesteps this with a different approach: full-featured browser tabs on a canvas. The actual website, product, or prototype lives spatially alongside notes, images, and drawings making it easy to build and explore in one place.

All this stuff is designed with collaboration in mind, so it's easy to vibe out designs, research, and more as you work with teams of people and agents.

<!-- TODO: Add screenshot or GIF demo here -->

## Some ways you can use this
1. Explore different directions and see them all side-by-side
2. Annotate live websites with feedback and pass that directly back to an agent
3. Create layouts with different device breakpoints to check responsiveness
4. Ask an agent to share its thinking visually
5. Add a repo and ingest its design system
6. Switch to browser mode for a more classic tab-based browser

## Key features

- **Canvas** — Arrange real browser windows on an infinite, zoomable canvas.
- **Agent-friendly** — Agents drive the canvas through a `specular` CLI (primary) or an MCP server (fallback) — creating frames, navigating, inspecting the DOM, clicking, typing, and taking screenshots
- **Agent presence** — See an agent's live cursor and task status as it works alongside you
- **Commenting & annotations** — Create annotations on any frame, usable by people and agents
- **Device frames** — Preview sites at preset device sizes (iPhone, iPad, Laptop, etc.) with visual device shells
- **Groups & layout** — Organize frames into groups with freeform, row, or grid layout modes
- **Edges & connections** — Draw connections between entities on the canvas
- **Undo/redo** — Full undo history backed by Yjs CRDTs
- **Local-first storage** — No sign-in, no account. All files live on your computer
- **Open file formats** — Canvas layout uses the [JSON Canvas](https://jsoncanvas.org) spec (also used by Obsidian); text and media are plain `.md`, `.png`, and `.webm` files that live on your computer in a folder.

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

Specular is designed to be driven by agents as a first-class collaborator. There are two ways in:

### CLI (primary)

The `specular` CLI is the main interface for agents. It exposes the full canvas surface — creating and arranging frames, snapshotting the DOM, clicking and filling fields, leaving annotations, and more — as composable commands that fit naturally into an agent's working loop.

```bash
specular workspace                       # inspect the current canvas
specular create frame <url>              # pull a live page onto the canvas
specular snapshot -i                     # get element refs for the selected frame
specular annotate "<feedback>"           # leave a comment for a human or agent
```

A Claude Code skill ships with the app so agents know how to use it. See [`resources/skills/specular/SKILL.md`](resources/skills/specular/SKILL.md) for the full command surface.

### MCP server (fallback)

An MCP server is also available for clients that prefer the Model Context Protocol. It covers the same core operations as the CLI. See the [MCP tools source](src/main/mcp-tools.ts) for the tool list.

## Security

To report a security vulnerability, see [SECURITY.md](SECURITY.md).

## License

Licensed under [PolyForm Shield 1.0.0](LICENSE.md). Copyright © 2026 Lyle Klyne.
