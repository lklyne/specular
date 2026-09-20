// ---------------------------------------------------------------------------
// MCP tool schema definitions
// ---------------------------------------------------------------------------

/** Every JSON Canvas spacing token (see docs/adr/0019, `layout-directive.ts`) — the
 *  pixel escape hatch (a plain number) is always accepted alongside these. */
const SPACING_TOKENS = ['xs', 's', 'm', 'l', 'xl']

const SPACING_VALUE_SCHEMA = {
  oneOf: [
    { type: 'number' },
    { type: 'string', enum: SPACING_TOKENS },
  ],
  description: 'Pixel number, or a spacing token (xs=20, s=40, m=60, l=100, xl=160).',
} as const

/** Shared by `upsert_entities` and `apply_patch` — a declarative reflow
 *  directive (ADR 0019) rather than per-item canvasX/Y. */
const LAYOUT_DIRECTIVE_SCHEMA = {
  type: 'object',
  description:
    "Declarative placement directive, applied to the patch's create items and any items with an id (re-laid-out). Without an anchor (originX/Y or near), it anchors at the bounding box of any existing entities in the patch, falling back to find_placement.",
  properties: {
    kind: { type: 'string', enum: ['row', 'column', 'grid'] },
    gap: SPACING_VALUE_SCHEMA,
    rowGap: SPACING_VALUE_SCHEMA,
    colGap: SPACING_VALUE_SCHEMA,
    cols: { type: 'integer', minimum: 1, description: 'Column count (grid only).' },
    originX: { type: 'number', description: 'Explicit anchor — must be given with originY.' },
    originY: { type: 'number', description: 'Explicit anchor — must be given with originX.' },
    near: { type: 'string', description: 'Anchor near an existing entity id.' },
  },
  required: ['kind'],
  additionalProperties: false,
} as const

/** Every tool takes an optional `tab` so an agent can target another canvas
 *  without switching what the user is looking at (see `switch_tab`) — added
 *  here rather than copy-pasted onto every schema below. Resolution and
 *  per-verb tab support (some routes 400 on a tab that doesn't apply to them,
 *  matching the CLI's `--tab`) live server-side; this only advertises the arg. */
const TAB_ARG_SCHEMA = {
  type: 'string',
  description:
    "Tab id or name to target instead of the canvas the user is looking at. Does not switch the user's focus — see switch_tab for that.",
} as const

function withTabArg<T extends { inputSchema: { properties: Record<string, unknown> } }>(tool: T): T {
  return {
    ...tool,
    inputSchema: {
      ...tool.inputSchema,
      properties: { ...tool.inputSchema.properties, tab: TAB_ARG_SCHEMA },
    },
  }
}

const rawToolSchemas = [
  {
    name: 'get_workspace',
    description: 'Return the current Specular workspace graph, selection, and occupied regions. Text entities include a preview of the first 80 characters — use get_text_entities for full content.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'get_selection',
    description: 'Return the current page or group selection.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'find_placement',
    description: 'Find a non-overlapping placement for a proposed cluster.',
    inputSchema: {
      type: 'object',
      properties: {
        width: { type: 'number' },
        height: { type: 'number' },
        anchor: {
          type: 'string',
          enum: ['selection_or_empty_region', 'empty_region'],
        },
      },
      required: ['width', 'height'],
      additionalProperties: false,
    },
  },
  {
    name: 'apply_task_layout',
    description: 'Create a breakpoint cluster for a URL using mobile, tablet, and desktop presets.',
    inputSchema: {
      type: 'object',
      properties: {
        task_kind: { type: 'string', enum: ['breakpoint_map'] },
        url: { type: 'string' },
        presets: {
          type: 'array',
          items: { type: 'string' },
        },
        anchor: {
          type: 'string',
          enum: ['selection_or_empty_region', 'empty_region'],
        },
        focus: { type: 'boolean' },
        label: { type: 'string' },
      },
      required: ['url'],
      additionalProperties: false,
    },
  },
  {
    name: 'upsert_entities',
    description: `Create or update canvas entities (pages, text notes, file attachments) in a single call.
If id matches an existing entity → update. No id → create.

Kind-specific fields:
  page — url, presetIndex, canvasX, canvasY, orientation, showDeviceFrame (default true), groupId
  text  — text (Markdown), color (hex "#RRGGBB" or preset 1-6: red/orange/yellow/green/cyan/purple), canvasX, canvasY, width, height
  file  — file (absolute path), subpath, canvasX, canvasY, width, height, presetIndex, orientation, showDeviceFrame (default false — set true to add a border/device frame, e.g. on an html entity)
  group — layoutGap (packing gap in px, managed auto-layout groups only)

Page presets (presetIndex → device):
  0: iPhone SE (375×667, mobile)     3: iPad Mini (744×1133)       6: Laptop (1280×800)
  1: iPhone 14 Pro (393×852, mobile) 4: iPad Pro 11 (834×1194)     7: Desktop (1440×900)
  2: iPhone 14 Pro Max (430×932)     5: iPad Pro 12.9 (1024×1366)  8: Desktop XL (1920×1080)
Portrait dimensions for phones/tablets. Use orientation: "landscape" to swap.

Pass \`layout\` to place items with a directive (row/column/grid) instead of explicit canvasX/Y — it overrides per-item positions for every item in the call, including updates by id.`,
    inputSchema: {
      type: 'object',
      properties: {
        items: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              kind: { type: 'string', enum: ['page', 'text', 'file'], description: 'Entity type. Required for creates; omit when updating by id — kind is resolved from the doc.' },
              id: { type: 'string', description: 'Entity ID. Present = update, absent = create.' },
              canvasX: { type: 'number' },
              canvasY: { type: 'number' },
              width: { type: 'number' },
              height: { type: 'number' },
              // Page
              url: { type: 'string' },
              presetIndex: { type: 'number', description: 'Device preset index (0-8).' },
              orientation: { type: 'string', enum: ['portrait', 'landscape'] },
              showDeviceFrame: { type: 'boolean', description: 'Show device bezel. Default true for new pages.' },
              groupId: { type: 'string' },
              // Text
              text: { type: 'string', description: 'Markdown content (text entity).' },
              color: { type: 'string', description: 'Background color (text entity).' },
              // File
              file: { type: 'string', description: 'Absolute file path (file entity).' },
              subpath: { type: 'string', description: 'Subpath within file (file entity).' },
              // Group
              layoutGap: { type: 'number', description: 'Packing gap in px (managed auto-layout group update only).' },
            },
            additionalProperties: true,
          },
        },
        layout: LAYOUT_DIRECTIVE_SCHEMA,
      },
      required: ['items'],
      additionalProperties: false,
    },
  },
  {
    name: 'apply_patch',
    description: `The declarative JSON door for batch canvas mutations (ADR 0019) — entities, edges, and deletes land in one Y.Doc transaction. Prefer the ergonomic tools (upsert_entities, link_pages, delete_entities) for the common single-purpose call; reach for apply_patch for the genuinely batch case ("create 6 pages in a 3x2 grid") or when entities, edges, and deletes need to land together atomically.

No id on an entity → create. id present → update (kind is resolved from the doc, no need to pass it). id listed in delete → removed. \`layout\` places/reflows the entities in this same patch — see upsert_entities for its shape.`,
    inputSchema: {
      type: 'object',
      properties: {
        entities: {
          type: 'array',
          description: 'Entities to create or update — same per-item shape as upsert_entities.items.',
          items: { type: 'object', additionalProperties: true },
        },
        edges: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              id: { type: 'string' },
              fromEntityId: { type: 'string' },
              toEntityId: { type: 'string' },
              kind: { type: 'string', enum: ['breakpoint_variant', 'connection'] },
              label: { type: 'string' },
            },
            required: ['fromEntityId', 'toEntityId'],
            additionalProperties: true,
          },
        },
        delete: {
          type: 'array',
          items: { type: 'string' },
          description: 'Entity or edge ids to remove.',
        },
        layout: LAYOUT_DIRECTIVE_SCHEMA,
      },
      additionalProperties: false,
    },
  },
  {
    name: 'link_pages',
    description: 'Create edges (connections) between any canvas entities (pages, text blocks, file blocks).',
    inputSchema: {
      type: 'object',
      properties: {
        edges: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              id: { type: 'string' },
              fromEntityId: { type: 'string' },
              toEntityId: { type: 'string' },
              kind: { type: 'string', enum: ['breakpoint_variant'] },
            },
            required: ['fromEntityId', 'toEntityId', 'kind'],
            additionalProperties: true,
          },
        },
      },
      required: ['edges'],
      additionalProperties: false,
    },
  },
  {
    name: 'unlink_pages',
    description: 'Delete semantic links by edge ID.',
    inputSchema: {
      type: 'object',
      properties: {
        edgeIds: {
          type: 'array',
          items: { type: 'string' },
        },
      },
      required: ['edgeIds'],
      additionalProperties: false,
    },
  },
  {
    name: 'focus_pages',
    description: 'Focus the canvas camera on pages, groups, or explicit bounds.',
    inputSchema: {
      type: 'object',
      properties: {
        pageIds: {
          type: 'array',
          items: { type: 'string' },
        },
        groupIds: {
          type: 'array',
          items: { type: 'string' },
        },
        bounds: {
          type: 'object',
          properties: {
            x: { type: 'number' },
            y: { type: 'number' },
            width: { type: 'number' },
            height: { type: 'number' },
          },
          required: ['x', 'y', 'width', 'height'],
          additionalProperties: false,
        },
      },
      additionalProperties: false,
    },
  },
  {
    name: 'create_group',
    description: 'Create a group from existing canvas entities.',
    inputSchema: {
      type: 'object',
      properties: {
        entity_ids: {
          type: 'array',
          items: { type: 'string' },
        },
        label: { type: 'string' },
      },
      required: ['entity_ids'],
      additionalProperties: false,
    },
  },
  {
    name: 'ungroup_group',
    description: 'Ungroup a group and return its freed entity IDs.',
    inputSchema: {
      type: 'object',
      properties: {
        group_id: { type: 'string' },
      },
      required: ['group_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'delete_groups',
    description: 'Delete groups and optionally all member pages.',
    inputSchema: {
      type: 'object',
      properties: {
        group_ids: {
          type: 'array',
          items: { type: 'string' },
        },
        delete_member_pages: { type: 'boolean' },
        focus_after: { type: 'boolean' },
      },
      required: ['group_ids'],
      additionalProperties: false,
    },
  },
  {
    name: 'arrange_entities',
    description: 'Tidy existing entities into a row, column, or grid — the same engine behind the canvas popup toolbar. Default keeps the current footprint and evens the gaps; pass gap to pack tight to a fixed gap in reading order instead. Omit entity_ids to arrange the current selection.',
    inputSchema: {
      type: 'object',
      properties: {
        mode: { type: 'string', enum: ['row', 'column', 'grid'] },
        entity_ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'Entities to arrange. Omit to use the current selection.',
        },
        gap: { ...SPACING_VALUE_SCHEMA, description: 'Pack to a fixed gap instead of tidying in place. ' + SPACING_VALUE_SCHEMA.description },
        cols: { type: 'number', description: 'Column count (grid mode).' },
      },
      required: ['mode'],
      additionalProperties: false,
    },
  },
  {
    name: 'auto_layout',
    description: "Turn a selection of entities (or a single existing group) into a managed auto-layout row or column — the direction follows the selection's dominant axis, and children can be drag-reordered afterward. A single group id converts that group in place.",
    inputSchema: {
      type: 'object',
      properties: {
        entity_ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'Entities to group, or a single existing group id (starting with "group_") to convert in place.',
        },
        label: { type: 'string' },
        gap: { type: 'number', description: 'Packing gap in px.' },
      },
      required: ['entity_ids'],
      additionalProperties: false,
    },
  },
  {
    name: 'register_design_system',
    description:
      'Register a design system manifest with component and token definitions.',
    inputSchema: {
      type: 'object',
      properties: {
        manifest: {
          type: 'object',
          additionalProperties: true,
        },
      },
      required: ['manifest'],
      additionalProperties: false,
    },
  },
  {
    name: 'get_design_system',
    description: 'Return the currently registered design system manifest.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'layout_component_states',
    description:
      'Create a page grid showing different states for a design system component.',
    inputSchema: {
      type: 'object',
      properties: {
        component: { type: 'string' },
        url: { type: 'string' },
        vary: {
          type: 'array',
          items: { type: 'string' },
        },
        values: {
          type: 'object',
          additionalProperties: true,
        },
        states: {
          type: 'array',
          items: { type: 'string' },
        },
        tokens: {
          type: 'object',
          additionalProperties: { type: 'string' },
        },
        selector: { type: 'string' },
        anchor: {
          type: 'string',
          enum: ['selection_or_empty_region', 'empty_region'],
        },
        focus: { type: 'boolean' },
        label: { type: 'string' },
      },
      required: ['component', 'url', 'vary'],
      additionalProperties: false,
    },
  },
  {
    name: 'browse',
    description:
      'Run an agent-browser command inside a page. Handles CDP connection, presence animation, and per-page serialization automatically.\n\nCommon commands:\n- "snapshot" / "snapshot -i" — accessibility tree with element refs (@eN)\n- "snapshot -s \\"#main\\"" — scope snapshot to a CSS selector\n- "click @eN" — click an element\n- "fill @eN text" — fill an input\n- "type @eN text" — type into an element\n- "select @eN value" — select a dropdown option\n- "scroll down" / "scroll up" — scroll the page\n- "wait --load networkidle" — wait for page to settle\n- "get text" / "get url" — read page content\n- "screenshot" — capture a PNG image\n- "screenshot --annotate" — labeled screenshot with ref overlay (snapshot + screenshot in one)\n- "diff snapshot" — show changes since last snapshot (+/- format)\n- "find text \\"Sign In\\" click" — semantic locators (no refs needed)\n- "console" / "errors" — page diagnostics\n- "query-elements selector" — find elements by CSS selector\n\nCommand chaining: use "cmd1 && cmd2 && cmd3" to run multiple commands in a single call with shared element refs. Example: "snapshot -i && click @e3 && get url".\n\nMutation commands (click, fill, type, select) automatically return the current URL.\n\nAfter mutations, re-snapshot to get fresh refs — element refs are per-snapshot and become stale after DOM changes.',
    inputSchema: {
      type: 'object',
      properties: {
        page_id: {
          type: 'string',
          description: 'Page to interact with. Defaults to the selected page.',
        },
        command: {
          type: 'string',
          description: 'The agent-browser command to run (e.g. "snapshot -i", "click @e5", "fill @e12 hello").',
        },
        echo: {
          type: 'boolean',
          description: 'After a successful mutation, append a fresh interactive snapshot to the result.',
        },
      },
      required: ['command'],
      additionalProperties: false,
    },
  },
  {
    name: 'create_annotation',
    description:
      'Create a new user-visible comment thread on the canvas or an element. The anchor type discriminates element / canvas / page / region (ADR 0006).',
    inputSchema: {
      type: 'object',
      properties: {
        text: { type: 'string' },
        anchor: {
          type: 'object',
          additionalProperties: true,
          description:
            "Annotation anchor. Examples: { type: 'canvas', canvasX, canvasY }, { type: 'page', pageId, offsetX, offsetY }, { type: 'element', pageId, selector, elementPath?, boundingBox? }, { type: 'region', canvasRect: { x, y, width, height } }. Always pass a region as canvasRect; a marquee that grabbed page content is stored page-relative internally so it scroll-follows.",
        },
        metadata: {
          type: 'object',
          additionalProperties: true,
        },
      },
      required: ['text', 'anchor'],
      additionalProperties: false,
    },
  },
  {
    name: 'get_annotations',
    description:
      'Return lightweight annotation stubs (comments, feedback) left by the user on the canvas. Includes summaries but not full metadata — use get_annotation_detail to fetch screenshots, DOM elements, and inspect context for a specific annotation.',
    inputSchema: {
      type: 'object',
      properties: {
        status: {
          type: 'string',
          enum: ['pending', 'acknowledged', 'resolved', 'dismissed'],
          description: 'Filter by annotation status. Omit to get all.',
        },
        url: {
          type: 'string',
          description: 'Filter annotations by canonical page URL.',
        },
        page_id: {
          type: 'string',
          description: 'Filter annotations by page id.',
        },
      },
      additionalProperties: false,
    },
  },
  {
    name: 'get_annotation_detail',
    description:
      'Get full detail for a single annotation including screenshot, DOM elements, computed styles, and component context. Use after get_annotations to drill into a specific annotation.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: {
          type: 'string',
          description: 'The annotation ID to fetch detail for.',
        },
        include_screenshot: {
          type: 'boolean',
          description:
            'Whether to include the region screenshot image. Defaults to true.',
        },
      },
      required: ['annotation_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'acknowledge_annotation',
    description:
      'Mark an annotation as acknowledged — you have seen and understood the feedback.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: { type: 'string' },
      },
      required: ['annotation_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'resolve_annotation',
    description:
      'Mark an annotation as resolved — the requested change has been made.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: { type: 'string' },
      },
      required: ['annotation_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'dismiss_annotation',
    description:
      'Dismiss an annotation with a reason — you have decided not to address this feedback.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: { type: 'string' },
        reason: { type: 'string' },
      },
      required: ['annotation_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'reply_to_annotation',
    description:
      'Add a reply to an annotation thread — ask a clarifying question or provide an update.',
    inputSchema: {
      type: 'object',
      properties: {
        annotation_id: { type: 'string' },
        text: { type: 'string' },
      },
      required: ['annotation_id', 'text'],
      additionalProperties: false,
    },
  },
  {
    name: 'annotate_selection',
    description:
      "Leave one region comment over a multi-selection's union bounds, carrying the selected entity ids so a fix loop reads the whole request at once. Omit entity_ids to use the current selection.",
    inputSchema: {
      type: 'object',
      properties: {
        text: { type: 'string' },
        entity_ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'Entities the comment is about. Omit to use the current selection.',
        },
      },
      required: ['text'],
      additionalProperties: false,
    },
  },
  {
    name: 'get_text_entities',
    description: 'Return all text entities on the canvas.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'get_file_entities',
    description: 'Return all file entities (images, attachments) on the canvas.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'start_recording',
    description:
      'Start a composite video recording of a page. Captures the page content with agent cursor overlay composited on top. Output is a VP9 WebM file.',
    inputSchema: {
      type: 'object',
      properties: {
        page_id: { type: 'string', description: 'Page to record.' },
        output_path: { type: 'string', description: 'Optional output file path. Defaults to a temp directory.' },
        fps: { type: 'number', description: 'Frame rate of the output file (default 30). Rates above 30 usually cannot be sustained and drop time.' },
        quality: {
          type: 'string',
          enum: ['high', 'medium', 'compact'],
          description: 'Quality preset. All record at 30fps; they differ in compression: high=crf20, medium=crf30, compact=crf40.',
        },
      },
      required: ['page_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'stop_recording',
    description:
      'Stop the current composite video recording. Returns the output path, duration, page count, and activity segments.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'get_recording_status',
    description:
      'Get the current recording state (idle or recording), including elapsed time and page count.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'trim_recording',
    description:
      'Trim idle segments from a recorded video. Uses activity segments to remove or speed up periods where the agent was idle.',
    inputSchema: {
      type: 'object',
      properties: {
        input_path: { type: 'string', description: 'Path to the recorded WebM file.' },
        output_path: { type: 'string', description: 'Optional output path for the trimmed file.' },
        min_idle_ms: { type: 'number', description: 'Minimum idle duration (ms) before trimming. Default: 3000.' },
        idle_speed_factor: { type: 'number', description: 'Speed multiplier for idle segments (e.g. 4 = 4x speed). Default: 4.' },
      },
      required: ['input_path'],
      additionalProperties: false,
    },
  },
  {
    name: 'print_pdf',
    description: 'Print a page to a PDF file on disk.',
    inputSchema: {
      type: 'object',
      properties: {
        page_id: { type: 'string' },
        output_path: { type: 'string', description: 'Defaults to "./<page_id>.pdf".' },
        landscape: { type: 'boolean' },
        page_size: { type: 'string', description: 'e.g. "Letter", "A4".' },
      },
      required: ['page_id'],
      additionalProperties: false,
    },
  },
  {
    name: 'delete_entities',
    description:
      'Delete a batch of mixed canvas entities (pages, text notes, file attachments) in a single call. Items are removed sequentially with animated cursor movement.',
    inputSchema: {
      type: 'object',
      properties: {
        items: {
          type: 'array',
          description: 'Array of entities to delete. Each must include `kind` and `id`.',
          items: {
            type: 'object',
            properties: {
              kind: { type: 'string', enum: ['page', 'text', 'file'], description: 'Entity type' },
              id: { type: 'string', description: 'Entity ID to delete' },
            },
            required: ['kind', 'id'],
          },
        },
      },
      required: ['items'],
      additionalProperties: false,
    },
  },
  {
    name: 'list_tabs',
    description: 'List every canvas (tab) in the workspace, marking the one the user is looking at.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
  {
    name: 'create_tab',
    description: 'Create a new canvas (tab) without switching the user\'s focus to it. Returns its id — pass that as `tab` on other tools to write to it in the background.',
    inputSchema: {
      type: 'object',
      properties: {
        name: { type: 'string' },
      },
      required: ['name'],
      additionalProperties: false,
    },
  },
  {
    name: 'switch_tab',
    description: 'Switch the canvas the user is looking at. This moves the user\'s focus — for writing to another canvas without disrupting them, pass `tab` on the other tools instead.',
    inputSchema: {
      type: 'object',
      properties: {
        ref: { type: 'string', description: 'Tab id or exact tab name.' },
      },
      required: ['ref'],
      additionalProperties: false,
    },
  },
  {
    name: 'delete_tab',
    description: 'Delete a canvas (tab). Deleting a background tab does not move the user; deleting the last tab resets it to an empty default canvas instead of removing it.',
    inputSchema: {
      type: 'object',
      properties: {
        ref: { type: 'string', description: 'Tab id or exact tab name.' },
      },
      required: ['ref'],
      additionalProperties: false,
    },
  },
  {
    name: 'start_task',
    description:
      "Bracket a multi-step task so the agent's cursor holds its position and stays visible on the canvas between calls instead of going idle. Call once before a sequence of related tool calls (building out a page, working through a checklist); always call finish_task when the task ends, whether it succeeded or not.",
    inputSchema: {
      type: 'object',
      properties: {
        label: { type: 'string', description: 'Short human-readable description of the task, shown next to the cursor.' },
      },
      required: ['label'],
      additionalProperties: false,
    },
  },
  {
    name: 'finish_task',
    description: 'End a task started with start_task, releasing the cursor hold so it can go idle again.',
    inputSchema: {
      type: 'object',
      properties: {},
      additionalProperties: false,
    },
  },
]

export const toolSchemas = rawToolSchemas.map(withTabArg)
