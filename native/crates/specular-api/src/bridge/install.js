(() => {
  const binding = window[__BINDING__];
  // No frame keeps the binding as it is: only the closure below calls it.
  try { delete window[__BINDING__]; } catch (_) {}
  if (typeof binding !== 'function' || window.top !== window
      || location.protocol !== 'file:' || window.specular) return;
  const nonce = __NONCE__;
  const waiting = new Map();
  const listeners = new Map();
  let next = 0;
  const send = (op, fields) => new Promise((resolve, reject) => {
    next += 1;
    waiting.set(next, { resolve, reject });
    binding(JSON.stringify({ ...fields, id: next, op, nonce }));
  });
  const receive = (message) => {
    if (message.id === undefined) {
      for (const callback of listeners.get(message.key) || []) {
        try { callback(message.value); } catch (error) { console.error(error); }
      }
      return;
    }
    const entry = waiting.get(message.id);
    if (!entry) return;
    waiting.delete(message.id);
    if (message.ok) entry.resolve(message.value);
    else entry.reject(new Error(message.error));
  };
  Object.defineProperty(window, '__specularReceive', { value: receive });
  const subscribe = (key, callback) => {
    let held = listeners.get(key);
    if (!held) {
      held = new Set();
      listeners.set(key, held);
    }
    held.add(callback);
    const off = () => {
      if (!held.delete(callback) || held.size) return;
      listeners.delete(key);
      send('unsubscribe', { key }).catch(() => {});
    };
    send('subscribe', { key }).then(
      (value) => { if (held.has(callback)) callback(value); },
      (error) => { off(); console.error(error); },
    );
    return off;
  };
  const specular = Object.freeze({
    state: (key) => send('state', { key }),
    run: (name, args) => send('run', { name, args: args === undefined ? null : args }),
    commands: () => send('commands', {}),
    subscribe,
  });
  Object.defineProperty(window, 'specular', { value: specular, enumerable: true });
  window.dispatchEvent(new Event('specular-ready'));
})();
