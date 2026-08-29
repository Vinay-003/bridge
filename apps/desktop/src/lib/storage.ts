export type StorageState = "IDLE"|"SCANNING"|"SYNCING"|"CONFLICT"|"DONE"
export type VectorClock = Record<string, number>

export function isValidStoragePath(p: string): boolean {
  if (!p || p.length === 0) return false
  if (p.length > 4096) return false
  if (p.includes('\0')) return false
  if (p === "/") return true
  // check segments
  const parts = p.split('/')
  for (const seg of parts) {
    if (seg === "" || seg === ".") continue
    if (seg === "..") return false
    if (seg.length > 255) return false
  }
  // additional: reject any ".." substring not as segment but also catch "/a/../b" already via seg, but also "a..b" is allowed?
  // We already reject seg == "..", so a..b passes.
  // But to be strict, if path contains ".." as segment, we already returned false.
  // Also need to catch "/a/../../b" => seg loop catches.
  return true
}

export function sanitizeStoragePath(p: string): string {
  if (!isValidStoragePath(p)) throw new Error(`invalid path: ${p}`)
  if (p === "/" || p === "") return ""
  const parts: string[] = []
  for (const seg of p.split('/')) {
    if (seg === "" || seg === ".") continue
    if (seg === "..") throw new Error(`path traversal: ${p}`)
    parts.push(seg)
  }
  return parts.join("/")
}

export function canTransitionStorage(from: StorageState, to: StorageState): boolean {
  const allowed: Record<StorageState, StorageState[]> = {
    IDLE: ["SCANNING","DONE"],
    SCANNING: ["SYNCING","DONE"],
    SYNCING: ["CONFLICT","DONE","IDLE"],
    CONFLICT: ["SYNCING","IDLE"],
    DONE: ["IDLE"],
  }
  return (allowed[from] || []).includes(to)
}

export function dominatesVector(a: VectorClock, b: VectorClock): boolean {
  let allGe = true
  let strictlyGreater = false
  for (const [k,bv] of Object.entries(b)) {
    const av = a[k] ?? 0
    if (av < bv) { allGe = false; break }
    if (av > bv) strictlyGreater = true
  }
  if (!allGe) return false
  for (const [k,av] of Object.entries(a)) {
    if (!(k in b) && av > 0) { strictlyGreater = true; break }
  }
  return strictlyGreater
}

export function isVectorConcurrent(a: VectorClock, b: VectorClock): boolean {
  if (equalVector(a,b)) return false
  return !dominatesVector(a,b) && !dominatesVector(b,a)
}
function equalVector(a: VectorClock, b: VectorClock): boolean {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)])
  for (const k of keys) {
    if ((a[k] ?? 0) !== (b[k] ?? 0)) return false
  }
  return true
}

export function mergeVector(a: VectorClock, b: VectorClock): VectorClock {
  const out: VectorClock = { ...a }
  for (const [k,bv] of Object.entries(b)) {
    out[k] = Math.max(out[k] ?? 0, bv)
  }
  return out
}

export type StorageChunk = {
  id: string
  path: string
  size: number
  offset: number
  total: number
  index: number
  sha256: string
  data_b64: string
}

export const CHUNK_SIZE = 1024 * 1024

export async function chunkStorageFile(id: string, name: string, buf: ArrayBuffer): Promise<StorageChunk[]> {
  const data = new Uint8Array(buf)
  const size = data.length
  if (size === 0) return []
  const total = Math.ceil(size / CHUNK_SIZE)
  const out: StorageChunk[] = []
  for (let idx = 0; idx < total; idx++) {
    const offset = idx * CHUNK_SIZE
    const slice = data.slice(offset, offset + CHUNK_SIZE)
    const hashBuf = await crypto.subtle.digest("SHA-256", slice)
    const sha256 = Array.from(new Uint8Array(hashBuf)).map(b=>b.toString(16).padStart(2,"0")).join("")
    // base64 encode
    let b64 = ""
    // slice may be large 1MB, use chunked conversion to avoid stack overflow with spread
    const chunkSize = 8192
    for (let i=0; i<slice.length; i+=chunkSize) {
      const sub = slice.subarray(i, i+chunkSize)
      b64 += String.fromCharCode(...sub)
    }
    b64 = btoa(b64)
    out.push({ id, path: name, size, offset, total, index: idx, sha256, data_b64: b64 })
  }
  return out
}

export function parseTrashInfo(content: string): Record<string,string> {
  const out: Record<string,string> = {}
  for (const line of content.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('[Trash') || trimmed.startsWith('#')) continue
    const eq = trimmed.indexOf('=')
    if (eq > 0) {
      const k = trimmed.slice(0, eq).trim()
      const v = trimmed.slice(eq+1).trim()
      out[k] = v
    }
  }
  return out
}

export function formatTrashPath(originalPath: string, deletionDate: string): string {
  return `[Trash Info]\nPath=${originalPath}\nDeletionDate=${deletionDate}\n`
}

export function trashInfoPath(trashFileName: string): string {
  return `${trashFileName}.trashinfo`
}

// helpers for UI
export type StorageEntry = {
  name: string
  path: string
  isDir: boolean
  size: number
  mtimeMs: number
  mime?: string
}

export function sortEntries(entries: StorageEntry[]): StorageEntry[] {
  return [...entries].sort((a,b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1
    return a.name.localeCompare(b.name)
  })
}

export function validateStorageSyncPayload(p: any): string | null {
  if (!p || typeof p.path !== "string") return "missing path"
  if (!isValidStoragePath(p.path)) return `invalid path: ${p.path}`
  if (p.path === "/") return "cannot sync root"
  if (typeof p.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(p.sha256)) return `invalid sha256: ${p.sha256}`
  if (typeof p.offset !== "number" || typeof p.size !== "number") return "missing offset/size"
  if (p.offset >= p.size && p.size !== 0) return `offset ${p.offset} >= size ${p.size}`
  if (typeof p.total !== "number" || typeof p.index !== "number") return "missing total/index"
  if (p.index >= p.total) return `index ${p.index} >= total ${p.total}`
  if (p.size > 50 * 1024 * 1024 * 1024) return "size > 50GiB"
  return null
}
