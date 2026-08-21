/**
 * no-bridge-event-handlers
 *
 * Forbids passing preload bridge functions (`somethingApi.method`) directly
 * as JSX event-handler props in src/renderer/.
 *
 * React calls event handlers with the SyntheticEvent as an argument, and
 * contextBridge deep-serializes every argument across the isolation
 * boundary. The event graph reaches `nativeEvent.view` → `window`, so one
 * click serializes the entire window object (~100k bridge ops), and the
 * first read of `window.speechSynthesis` blocks the browser process for
 * ~800ms enumerating macOS voices. Wrap the call in an arrow with explicit
 * arguments instead: `onClick={() => api.method()}`.
 */
'use strict'

const HANDLER_PROP = /^on[A-Z]/
const BRIDGE_OBJECT = /(?:^a|A)pi$/

module.exports = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'Disallow bridge functions as JSX event-handler props. Wrap in an arrow with explicit arguments.',
    },
    messages: {
      bridgeHandler:
        "'{{prop}}={{{source}}}' passes a bridge function directly as an event handler, so the event object gets serialized across the contextBridge. Wrap it: '{{prop}}={() => {{source}}(…)}'.",
    },
    schema: [],
  },
  create(context) {
    const filename = context.filename ?? context.getFilename()
    if (!filename.includes('/src/renderer/')) return {}

    return {
      JSXAttribute(node) {
        if (node.name.type !== 'JSXIdentifier') return
        const prop = node.name.name
        if (!HANDLER_PROP.test(prop)) return
        const value = node.value
        if (value?.type !== 'JSXExpressionContainer') return
        const expr = value.expression
        if (expr.type !== 'MemberExpression') return
        if (expr.object.type !== 'Identifier') return
        if (!BRIDGE_OBJECT.test(expr.object.name)) return
        const property =
          expr.property.type === 'Identifier' ? expr.property.name : '…'
        context.report({
          node: value,
          messageId: 'bridgeHandler',
          data: { prop, source: `${expr.object.name}.${property}` },
        })
      },
    }
  },
}
