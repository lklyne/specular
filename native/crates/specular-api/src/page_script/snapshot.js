// The page's DOM as the Electron app's `takeDomSnapshot` gives it: an
// indented outline, or with `structured` a flat list of nodes with refs.
// Called as `(this script)(maxDepth, structured)`; returns plain JSON.
(function (maxDepth, structured) {
  function compactText(value, max) {
    if (!value) return undefined;
    const cleaned = value.replace(/\s+/g, ' ').trim();
    if (!cleaned) return undefined;
    return cleaned.length > max ? cleaned.slice(0, max - 1) + '…' : cleaned;
  }
  function bestName(element) {
    const text =
      compactText(element.innerText, 80) ??
      compactText(element.getAttribute('aria-label'), 120) ??
      compactText(element.getAttribute('title'), 120) ??
      compactText(element.value, 120) ??
      compactText(element.getAttribute('placeholder'), 120) ??
      compactText(element.getAttribute('alt'), 120);
    const tag = element.tagName.toLowerCase();
    return text ? tag + ' "' + text + '"' : tag;
  }
  function segment(element) {
    const tag = element.tagName.toLowerCase();
    const id = element.getAttribute('id');
    if (id) return tag + '#' + id;
    const role = element.getAttribute('role');
    if (role) return tag + '[role="' + role + '"]';
    const classes = [...element.classList].map((c) => c.trim()).filter(Boolean).slice(0, 2);
    return classes.length ? tag + '.' + classes.join('.') : tag;
  }
  function elementPath(element, depthLimit) {
    const segments = [];
    let current = element;
    let depth = 0;
    while (current && depth < depthLimit) {
      segments.unshift(segment(current));
      const root = current.getRootNode();
      if (root instanceof ShadowRoot && root.host instanceof Element) {
        segments.unshift('#shadow-root');
        current = root.host;
      } else {
        current = current.parentElement;
      }
      depth += 1;
    }
    return segments.join(' > ');
  }
  function visible(element) {
    const rect = element.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) return false;
    const styles = getComputedStyle(element);
    return styles.display !== 'none' && styles.visibility !== 'hidden';
  }
  const INTERACTIVE_TAGS = ['a', 'button', 'input', 'select', 'textarea', 'summary', 'option', 'label'];
  const INTERACTIVE_ROLES = ['button', 'link', 'checkbox', 'textbox', 'menuitem', 'option', 'tab', 'switch'];
  function interactive(element) {
    if (INTERACTIVE_TAGS.includes(element.tagName.toLowerCase())) return true;
    const role = element.getAttribute('role');
    if (role && INTERACTIVE_ROLES.includes(role)) return true;
    if (element.hasAttribute('onclick')) return true;
    if (element.tabIndex >= 0) return true;
    return getComputedStyle(element).cursor === 'pointer';
  }
  function outline(element, depth, indent) {
    if (depth > maxDepth || !visible(element)) return '';
    const rect = element.getBoundingClientRect();
    const role = element.getAttribute('role');
    const text = compactText(element.textContent, 80);
    let label = element.tagName.toLowerCase();
    if (role) label += ' role="' + role + '"';
    if (text) label += ' "' + text + '"';
    const box =
      '[' + Math.round(rect.left) + ',' + Math.round(rect.top) + ' ' +
      Math.round(rect.width) + 'x' + Math.round(rect.height) + ']';
    let result = indent + label + ' ' + box + ' path=' + elementPath(element, 4) + '\n';
    for (const child of element.children) result += outline(child, depth + 1, indent + '  ');
    return result;
  }
  if (!document.body) return structured ? { url: location.href, title: document.title, nodes: [] } : '';
  if (!structured) return outline(document.body, 0, '');
  const nodes = [];
  let next = 1;
  function walk(element, depth, parentRef) {
    if (depth > maxDepth || !visible(element)) return;
    const rect = element.getBoundingClientRect();
    const ref = '@e' + next;
    next += 1;
    nodes.push({
      ref,
      parentRef,
      depth,
      tagName: element.tagName.toLowerCase(),
      role: element.getAttribute('role') ?? undefined,
      name: bestName(element) || undefined,
      text: compactText(element.textContent, 80) || undefined,
      interactive: interactive(element),
      bounds: {
        x: Math.round(rect.left),
        y: Math.round(rect.top),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
      },
      elementPath: elementPath(element, 4),
      fullPath: elementPath(element, 10),
    });
    for (const child of element.children) walk(child, depth + 1, ref);
  }
  walk(document.body, 0, null);
  return { url: location.href, title: document.title, nodes };
})
