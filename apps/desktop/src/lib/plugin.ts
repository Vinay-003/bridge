export const ALLOWED_CAPS = ["notify","clipboard","storage","ai.summarize","ai.transcribe"] as const
export type Capability = typeof ALLOWED_CAPS[number]
export type PluginState = "UNLOADED"|"LOADING"|"LOADED"|"RUNNING"|"RELOADING"|"FAILED"|"DISABLED"

export type PluginManifest = {
  name: string
  version: string
  displayName?: string
  description?: string
  entry: string
  capabilities: Capability[]
  author?: string
  bridgeVersion?: string
}

export function isValidPluginId(s: string): boolean {
  if (!s || s.length<3 || s.length>32) return false
  return /^[a-z0-9-_]+$/.test(s)
}
export function isValidPluginVersion(s: string): boolean {
  const parts = s.split('.')
  if (parts.length!==3) return false
  for (const p of parts) {
    if (!p || p.length>5) return false
    if (!/^\d+$/.test(p)) return false
    if (p.length>1 && p.startsWith('0')) return false
  }
  return true
}
export function sanitizePluginPath(entry: string): string | null {
  if (!entry || entry.length===0 || entry.length>256) return "entry empty/too long"
  if (entry.includes('\0')) return "entry contains NUL"
  if (entry.startsWith('/') || entry.startsWith('\\')) return "entry must be relative"
  for (const seg of entry.split('/')) {
    if (seg === "..") return `path traversal: ${entry}`
    if (seg.includes('\\')) return "entry contains backslash"
  }
  if (!(entry.endsWith('.js') || entry.endsWith('.wasm'))) return "entry must end with .js or .wasm"
  return null
}
export function validatePluginManifest(m: any): string | null {
  if (!m || typeof m.name !== "string" || !isValidPluginId(m.name)) return `invalid plugin name: ${m?.name}`
  if (typeof m.version !== "string" || !isValidPluginVersion(m.version)) return `invalid version: ${m?.version}`
  if (typeof m.entry !== "string") return "missing entry"
  const err = sanitizePluginPath(m.entry)
  if (err) return err
  if (!Array.isArray(m.capabilities) || m.capabilities.length===0) return "capabilities empty"
  for (const c of m.capabilities) {
    if (!(ALLOWED_CAPS as readonly string[]).includes(c)) return `invalid capability: ${c}`
  }
  if (m.bridgeVersion !== undefined && m.bridgeVersion !== "1") return `invalid bridgeVersion: ${m.bridgeVersion}`
  return null
}
export function validatePluginLoadPayload(p: any): string | null {
  if (!p || typeof p.pluginId !== "string" || !isValidPluginId(p.pluginId)) return `invalid pluginId: ${p?.pluginId}`
  return null
}
export function canPluginAccess(caps: string[], needed: string): boolean {
  return caps.includes(needed)
}
export function canTransitionPlugin(from: PluginState, to: PluginState): boolean {
  const allowed: Record<PluginState, PluginState[]> = {
    UNLOADED: ["LOADING"],
    LOADING: ["LOADED","FAILED"],
    LOADED: ["RUNNING","FAILED"],
    RUNNING: ["RELOADING","FAILED","DISABLED","UNLOADED"],
    RELOADING: ["RUNNING","FAILED"],
    FAILED: ["LOADING","UNLOADED"],
    DISABLED: ["LOADING"],
  }
  return (allowed[from]||[]).includes(to)
}
export function mockTranslate(text: string, targetLang='en'): string {
  return text.split('').reverse().join('') + ` [translated:${targetLang}]`
}
