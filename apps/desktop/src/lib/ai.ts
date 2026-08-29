export type AiState = "IDLE"|"QUEUED"|"LOCAL"|"CLOUD"|"DONE"|"FAILED"
export type AiKind = "summarize"|"transcribe"

export function canTransitionAi(from: AiState, to: AiState): boolean {
  const allowed: Record<AiState, AiState[]> = {
    IDLE: ["QUEUED"],
    QUEUED: ["LOCAL","CLOUD","FAILED"],
    LOCAL: ["DONE","CLOUD","FAILED"],
    CLOUD: ["DONE","FAILED"],
    DONE: ["IDLE"],
    FAILED: ["IDLE"],
  }
  return (allowed[from]||[]).includes(to)
}

export function validateAiSummarizePayload(p: any): string | null {
  if (!p || !Array.isArray(p.notifications)) return "missing notifications"
  if (p.notifications.length===0 || p.notifications.length>20) return `notifications len ${p.notifications.length} invalid 1..20`
  let totalChars = 0
  for (const n of p.notifications) {
    const app = n.app as string ?? ""
    const body = n.body as string ?? ""
    if (!app || app.length===0 || app.length>64) return `invalid app: ${app}`
    if (body.length>500) return `body too long: ${body.length}`
    totalChars += app.length + body.length + 50
  }
  if (totalChars > 10*1024) return "total chars >10k"
  if (p.maxLen !== undefined && (p.maxLen===0 || p.maxLen>1000)) return `invalid maxLen: ${p.maxLen}`
  if (p.requestId !== undefined && (typeof p.requestId!=="string" || p.requestId.length===0 || p.requestId.length>64)) return "invalid requestId"
  return null
}

export function validateAiTranscribePayload(p: any): string | null {
  if (!p || typeof p.audio_b64 !== "string" || p.audio_b64.length===0 || p.audio_b64.length>7_000_000) return `invalid audio_b64 len: ${p.audio_b64?.length}`
  // base64 check
  try { atob(p.audio_b64) } catch { return "invalid audio_b64 base64"}
  // decode len check ≤5MB
  try {
    const dec = atob(p.audio_b64)
    if (dec.length > 5*1024*1024) return "audio decoded >5MB"
    if (dec.length===0) return "audio empty"
  } catch { return "invalid audio_b64 base64"}
  if (!["opus","wav","mp3","m4a"].includes(p.format)) return `invalid format: ${p.format}`
  if (p.lang !== undefined && !/^[a-z]{2}$/.test(p.lang)) return `invalid lang: ${p.lang}`
  if (p.requestId !== undefined && (typeof p.requestId!=="string" || p.requestId.length===0 || p.requestId.length>64)) return "invalid requestId"
  return null
}

export function validateAiResultPayload(p: any): string | null {
  if (!p || !["summarize","transcribe"].includes(p.kind)) return `invalid kind: ${p?.kind}`
  if (typeof p.text !== "string") return "missing text"
  if (p.text.length>5000) return "text too long"
  if (typeof p.model !== "string" || p.model.length===0 || p.model.length>64) return "invalid model"
  return null
}

export function shouldRateLimit(timestamps: number[], now: number, limit=10, windowMs=60000): boolean {
  while (timestamps.length && now - timestamps[0] > windowMs) timestamps.shift()
  if (timestamps.length >= limit) return true
  timestamps.push(now)
  return false
}

export function localSummarize(notifications: any[], maxLen=200): string {
  const perApp: Record<string, number> = {}
  for (const n of notifications) {
    const app = n.app ?? "unknown"
    perApp[app] = (perApp[app]??0)+1
  }
  const parts = Object.entries(perApp).map(([k,v]) => `${k}×${v}`).sort()
  let summary = `${notifications.length} notifications: ${parts.join(', ')}`
  if (notifications[0]?.body) {
    const snippet = notifications[0].body.slice(0,60)
    summary += ` — e.g., ${snippet}`
  }
  return summary.slice(0, maxLen)
}

export function localTranscribe(b64Len: number, format: string, lang: string): string {
  return `Transcribed ${b64Len} audio (format ${format}, lang ${lang}) — mock whisper.cpp local`
}
