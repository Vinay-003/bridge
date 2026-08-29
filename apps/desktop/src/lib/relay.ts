export type RelayState = "DISCONNECTED"|"ANNOUNCING"|"HOLE_PUNCHING"|"RELAY_READY"|"CONNECTED_DIRECT"|"CONNECTED_VIA_RELAY"|"FAILED"
export const RELAY_URL = "https://relay.bridge.dev/v1/announce"
export const STUN_SERVER = "stun.l.google.com:19302"

export function isValidDeviceId(s: string): boolean {
  if (!s || s.length === 0 || s.length > 64) return false
  return /^[a-zA-Z0-9._-]+$/.test(s)
}
export function isValidStunServer(s: string): boolean {
  const idx = s.lastIndexOf(':')
  if (idx < 1) return false
  const host = s.slice(0, idx)
  const portStr = s.slice(idx+1)
  if (!host || host.length > 253) return false
  const port = parseInt(portStr, 10)
  if (isNaN(port) || port <=0 || port > 65535) return false
  if (host.includes(' ') || host.includes('\0')) return false
  return true
}
export function isOpaqueBlob(s: string): boolean {
  if (!s || s.length < 16 || s.length > 1_400_000) return false
  if (s.includes('{') || s.includes('"')) return false
  return /^[A-Za-z0-9+/=]+$/.test(s)
}
export function canTransitionRelay(from: RelayState, to: RelayState): boolean {
  const allowed: Record<RelayState, RelayState[]> = {
    DISCONNECTED: ["ANNOUNCING","FAILED"],
    ANNOUNCING: ["HOLE_PUNCHING","RELAY_READY","FAILED","DISCONNECTED"],
    HOLE_PUNCHING: ["CONNECTED_DIRECT","RELAY_READY","FAILED"],
    RELAY_READY: ["CONNECTED_VIA_RELAY","DISCONNECTED"],
    CONNECTED_DIRECT: ["DISCONNECTED","RELAY_READY"],
    CONNECTED_VIA_RELAY: ["DISCONNECTED"],
    FAILED: ["DISCONNECTED"],
  }
  return (allowed[from]||[]).includes(to)
}
export function validateRelayAnnouncePayload(p: any): string | null {
  if (!p || typeof p.deviceId !== "string" || !isValidDeviceId(p.deviceId)) return "invalid deviceId"
  if (typeof p.blob !== "string" || !isOpaqueBlob(p.blob)) return "invalid blob"
  // decode check length
  try {
    const dec = atob(p.blob)
    if (dec.length > 1024*1024) return "blob too large"
    if (dec.length < 12) return "blob too small"
  } catch { return "invalid blob base64"}
  if (p.ts !== undefined) {
    const now = Date.now()
    if (Math.abs(now - p.ts) > 5*60*1000) return "clock skew"
  }
  if (p.stunServer !== undefined && !isValidStunServer(p.stunServer)) return `invalid stunServer: ${p.stunServer}`
  if (p.mappedAddr !== undefined) {
    // simple SocketAddr check ip:port
    if (!/^(\d+\.\d+\.\d+\.\d+|\[.+\]):\d+$/.test(p.mappedAddr) && p.mappedAddr.split(':').length <2) return `invalid mappedAddr: ${p.mappedAddr}`
    // try parse
    const lastCol = p.mappedAddr.lastIndexOf(':')
    const port = parseInt(p.mappedAddr.slice(lastCol+1),10)
    if (isNaN(port) || port<=0 || port>65535) return `invalid mappedAddr: ${p.mappedAddr}`
  }
  if (p.fp !== undefined && !/^[0-9a-f]{12}$/.test(p.fp)) return `invalid fp: ${p.fp}`
  return null
}
export function validateRelayRelayPayload(p: any): string | null {
  if (!p || typeof p.to !== "string" || !isValidDeviceId(p.to)) return "invalid to"
  if (typeof p.from !== "string" || !isValidDeviceId(p.from)) return "invalid from"
  if (typeof p.blob !== "string" || !isOpaqueBlob(p.blob)) return "invalid blob"
  try {
    const dec = atob(p.blob)
    if (dec.length > 1024*1024) return "blob too large"
  } catch { return "invalid blob base64"}
  if (p.ts !== undefined && Math.abs(Date.now()-p.ts) > 5*60*1000) return "clock skew"
  if (p.nonce !== undefined && !/^[0-9a-f]{8}$/.test(p.nonce)) return `invalid nonce: ${p.nonce}`
  return null
}
export function encodeStunBindingRequest(txid: Uint8Array): Uint8Array {
  if (txid.length !== 12) throw new Error("txid must be 12 bytes")
  const buf = new Uint8Array(20)
  buf[0]=0; buf[1]=0x01
  buf[2]=0; buf[3]=0
  buf[4]=0x21; buf[5]=0x12; buf[6]=0xA4; buf[7]=0x42
  buf.set(txid, 8)
  return buf
}
export function rateLimitCheck(timestamps: number[], now: number, limit=20, windowMs=60000): boolean {
  while (timestamps.length && now - timestamps[0] > windowMs) timestamps.shift()
  if (timestamps.length >= limit) return true
  timestamps.push(now)
  return false
}
