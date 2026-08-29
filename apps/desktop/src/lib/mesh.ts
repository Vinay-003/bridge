export type MeshState = "IDLE"|"SYNCING"|"CONFLICT"
export type VectorClock = Record<string, number>
export type LwwClipboard = { text: string, mime: string, ts: number, device_id: string }

export function canTransitionMesh(from: MeshState, to: MeshState): boolean {
  const allowed: Record<MeshState, MeshState[]> = {
    IDLE: ["SYNCING"],
    SYNCING: ["IDLE","CONFLICT"],
    CONFLICT: ["SYNCING","IDLE"],
  }
  return (allowed[from]||[]).includes(to)
}

export function isValidDeviceId(s: string): boolean {
  if (!s || s.length===0 || s.length>64) return false
  return /^[a-zA-Z0-9._-]+$/.test(s)
}
export function dominatesVector(a: VectorClock, b: VectorClock): boolean {
  let allGe = true
  let strictlyGreater = false
  for (const [k,bv] of Object.entries(b)) {
    const av = a[k] ?? 0
    if (av < bv) { allGe=false; break }
    if (av > bv) strictlyGreater = true
  }
  if (!allGe) return false
  for (const [k,av] of Object.entries(a)) {
    if (!(k in b) && av>0) { strictlyGreater=true; break }
  }
  return strictlyGreater
}
export function isVectorConcurrent(a: VectorClock, b: VectorClock): boolean {
  if (equalVector(a,b)) return false
  return !dominatesVector(a,b) && !dominatesVector(b,a)
}
function equalVector(a: VectorClock, b: VectorClock): boolean {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)])
  for (const k of keys) if ((a[k]??0) !== (b[k]??0)) return false
  return true
}
export function mergeVector(a: VectorClock, b: VectorClock): VectorClock {
  const out: VectorClock = {...a}
  for (const [k,bv] of Object.entries(b)) out[k]=Math.max(out[k]??0, bv)
  return out
}
export function isVectorMonotonic(local: VectorClock, incoming: VectorClock, deviceId: string): boolean {
  const localCnt = local[deviceId] ?? 0
  const incomingCnt = incoming[deviceId] ?? 0
  return incomingCnt <= localCnt + 1
}
export function lwwMerge(a: LwwClipboard, b: LwwClipboard): LwwClipboard {
  if (b.ts > a.ts) return b
  if (b.ts < a.ts) return a
  return b.device_id > a.device_id ? b : a
}
export function isValidStoragePath(p: string): boolean {
  if (!p || p.length===0) return false
  if (p.length>4096) return false
  if (p.includes('\0')) return false
  if (p === "/") return true
  const parts = p.split('/')
  for (const seg of parts) {
    if (seg===""|| seg==="." ) continue
    if (seg==="..") return false
    if (seg.length>255) return false
  }
  return true
}
export function validateMeshSyncPayload(p: any): string | null {
  if (!p || typeof p.deviceId !== "string" || !isValidDeviceId(p.deviceId)) return "missing/invalid deviceId"
  if (!p.vectors || typeof p.vectors !== "object") return "missing vectors"
  for (const [k,v] of Object.entries(p.vectors)) {
    if (!isValidDeviceId(k)) return `invalid vector key: ${k}`
    if (typeof v !== "number" || v<0) return `invalid vector value for ${k}`
  }
  if (p.entries !== undefined) {
    if (!Array.isArray(p.entries)) return "entries must be array"
    if (p.entries.length>100) return "entries >100"
    for (const e of p.entries) {
      if (!e.path || typeof e.path !== "string") return "entry missing path"
      if (!isValidStoragePath(e.path)) return `invalid path: ${e.path}`
      if (e.vector !== undefined) {
        for (const [k,v] of Object.entries(e.vector as any)) {
          if (!isValidDeviceId(k)) return `invalid entry vector key: ${k}`
          if (typeof v!=="number") return `invalid entry vector value`
        }
      }
      if (e.lww !== undefined) {
        if (typeof e.lww.text === "string" && e.lww.text.length>1024*1024) return "lww text too large"
        if (typeof e.lww.ts !== "number") return "missing lww ts"
        if (Math.abs(Date.now()-e.lww.ts) > 5*60*1000) return "lww clock skew"
      }
      if (e.sha256 !== undefined && !/^[0-9a-f]{64}$/.test(e.sha256)) return `invalid sha256: ${e.sha256}`
    }
  }
  if (p.ts !== undefined && Math.abs(Date.now()-p.ts) > 5*60*1000) return "clock skew"
  return null
}
export function validateMeshConflictPayload(p: any): string | null {
  if (!p || typeof p.path !== "string" || !isValidStoragePath(p.path)) return `invalid path: ${p?.path}`
  if (!["lww","rename","manual"].includes(p.resolution)) return `invalid resolution: ${p.resolution}`
  if (!["local","remote"].includes(p.winner)) return `invalid winner: ${p.winner}`
  if (p.loserRename !== undefined) {
    if (typeof p.loserRename !== "string") return "invalid loserRename"
    if (p.loserRename.includes("..")) return `invalid loserRename: ${p.loserRename}`
    if (p.loserRename.length>4096) return "loserRename too long"
  }
  return null
}
