export type InputAction = "tap"|"down"|"move"|"up"|"swipe"|"pinch"|"drag"|"key"|"home"|"back"
export type ControlState = "DISABLED"|"ENABLED"|"CONTROLLING"|"PAUSED"

export const ALLOWED_ACTIONS: InputAction[] = ["tap","down","move","up","swipe","pinch","drag","key","home","back"]

export function isValidAction(a: string): boolean {
  return (ALLOWED_ACTIONS as string[]).includes(a)
}

export function clamp01(v: number): number | null {
  if (!Number.isFinite(v)) return null
  if (v < 0 || v > 1) return null
  return v
}

export function normToPx(norm: number, sizePx: number): number {
  return Math.floor(norm * sizePx)
}

export function pxToNorm(px: number, sizePx: number): number {
  if (sizePx <= 0) return 0
  return px / sizePx
}

export function canTransition(from: ControlState, to: ControlState): boolean {
  const allowed: Record<ControlState, ControlState[]> = {
    DISABLED: ["ENABLED"],
    ENABLED: ["CONTROLLING","DISABLED"],
    CONTROLLING: ["PAUSED","ENABLED","DISABLED"],
    PAUSED: ["ENABLED","DISABLED"],
  }
  return (allowed[from] || []).includes(to)
}

export type InputEventPayload = {
  x?: number
  y?: number
  action: InputAction
  displayId?: number
  pointerId?: number
  pressure?: number
  durationMs?: number
  scale?: number
  keyCode?: number
  ts?: number
}

export function validateInputEvent(p: InputEventPayload): string | null {
  if (!isValidAction(p.action)) return `invalid action: ${p.action}`
  const needsCoords = !["home","back","key"].includes(p.action)
  if (p.action === "key") {
    if (p.keyCode === undefined || p.keyCode === null) return "key requires keyCode"
    if (p.keyCode < 0 || p.keyCode > 1000) return `invalid keyCode: ${p.keyCode}`
  }
  if (needsCoords) {
    if (p.x === undefined || p.y === undefined) return "missing x/y"
    if (clamp01(p.x) === null) return `invalid x: ${p.x}`
    if (clamp01(p.y) === null) return `invalid y: ${p.y}`
  } else {
    if (p.x !== undefined && clamp01(p.x) === null) return `invalid x: ${p.x}`
    if (p.y !== undefined && clamp01(p.y) === null) return `invalid y: ${p.y}`
  }
  if (p.pointerId !== undefined && (p.pointerId < 0 || p.pointerId > 9)) return `invalid pointerId: ${p.pointerId}`
  if (p.pressure !== undefined && (!Number.isFinite(p.pressure) || p.pressure < 0 || p.pressure > 1)) return `invalid pressure: ${p.pressure}`
  if (p.durationMs !== undefined && (p.durationMs < 0 || p.durationMs > 5000)) return `invalid durationMs: ${p.durationMs}`
  if (p.scale !== undefined) {
    if (p.action !== "pinch") return "scale only for pinch"
    if (!Number.isFinite(p.scale) || p.scale < 0.1 || p.scale > 5) return `invalid scale: ${p.scale}`
  }
  if (p.displayId !== undefined && p.displayId < 0) return `invalid displayId: ${p.displayId}`
  return null
}

export function buildInputEventPayload(p: InputEventPayload): InputEventPayload {
  const err = validateInputEvent(p)
  if (err) throw new Error(err)
  return { displayId: 0, pointerId: 0, ...p }
}

// Throttling: 60fps => 16ms
export function shouldThrottle(lastTs: number | null, now: number, throttleMs = 16): boolean {
  if (lastTs === null) return false
  return now - lastTs < throttleMs
}

// Coalesce moves within throttle window
export function coalesceMove(pending: InputEventPayload | null, incoming: InputEventPayload): InputEventPayload {
  if (pending && pending.action === "move" && incoming.action === "move") {
    return incoming
  }
  return incoming
}

// Display scaling: canvas letterbox
export function canvasToNorm(
  clientX: number,
  clientY: number,
  rect: { left: number; top: number; width: number; height: number },
  display: { width: number; height: number },
  letterbox = true
): { x: number; y: number } {
  // Simple: map clientX within rect to norm 0..1
  // If letterbox, adjust for aspect ratio letterboxing
  if (!letterbox) {
    const x = (clientX - rect.left) / rect.width
    const y = (clientY - rect.top) / rect.height
    return { x: Math.max(0, Math.min(1, x)), y: Math.max(0, Math.min(1, y)) }
  }
  // Letterbox: compute scaling to fit display aspect
  const canvasAspect = rect.width / rect.height
  const displayAspect = display.width / display.height
  let drawWidth = rect.width
  let drawHeight = rect.height
  let offsetX = 0
  let offsetY = 0
  if (displayAspect > canvasAspect) {
    // display wider than canvas: letterbox horizontal? actually height constrained
    drawWidth = rect.height * displayAspect
    offsetX = (rect.width - drawWidth) / 2
  } else if (displayAspect < canvasAspect) {
    drawHeight = rect.width / displayAspect
    offsetY = (rect.height - drawHeight) / 2
  }
  // If drawWidth/Height exceed rect, we center; else fill
  // Simpler: if letterboxed, map within draw area
  // If offset negative, it means draw extends beyond rect (crop), so clamp
  const xInDraw = clientX - rect.left - offsetX
  const yInDraw = clientY - rect.top - offsetY
  const x = xInDraw / drawWidth
  const y = yInDraw / drawHeight
  return { x: Math.max(0, Math.min(1, x)), y: Math.max(0, Math.min(1, y)) }
}

export function rateLimitCheck(timestamps: number[], now: number, limit = 120, windowMs = 1000): boolean {
  // returns true if rate limited
  while (timestamps.length && now - timestamps[0] > windowMs) timestamps.shift()
  if (timestamps.length >= limit) return true
  timestamps.push(now)
  return false
}

export function displayInfoToString(info: any): string {
  if (!info) return "no display"
  if (info.displays) return `${info.displays.length} displays, primary ${info.primaryDisplayId}`
  return `display ${info.displayId} ${info.width}x${info.height}`
}
