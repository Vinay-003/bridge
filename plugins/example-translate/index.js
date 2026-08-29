// Bridge Plugin: example-translate (mock)
// Manifest: bridge.json capabilities ["notify","clipboard"]
// This plugin listens to notify.new and appends [translated] then syncs via clipboard.
// In real wasmtime/deno sandbox, this would be executed with fuel limit and capability checks.
// Here as mock JS, daemon's plugin.rs validates capability before allowing bridge.* calls.

// Mock API surface (provided by daemon sandbox stub):
// bridge.on('notify.new', (payload)=>{...})
// bridge.clipboard.sync({text, mime})

// For demonstration, expose translate function:
function mockTranslate(text, targetLang = 'en') {
  // deterministic mock: reverse + [translated]
  return text.split('').reverse().join('') + ` [translated:${targetLang}]`;
}

// Simulated event handler (called by daemon stub in tests)
function onNotifyNew(payload) {
  if (!payload || !payload.body) return null;
  const translated = mockTranslate(payload.body, 'en');
  // In real sandbox, capability check for clipboard must pass
  // daemon would handle bridge.clipboard.sync with capability gate
  return {
    text: translated,
    mime: 'text/plain',
    source: 'plugin:example-translate',
    ts: Date.now()
  };
}

if (typeof module !== 'undefined') {
  module.exports = { mockTranslate, onNotifyNew };
}
