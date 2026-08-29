export function isValidNumber(n: string): boolean {
  const digits = n.replace(/[^0-9]/g, '')
  return digits.length >= 7 && digits.length <= 15 && /^[+0-9 ()-]+$/.test(n.trim()) && n.trim().length > 0
}
export function isValidSmsBody(s: string): boolean {
  return s.length > 0 && s.length <= 918 && s.trim().length > 0
}
export type CallState = "IDLE"|"RINGING"|"OFFHOOK"|"HUNGUP"
export function canTransition(from: CallState, to: CallState): boolean {
  const allowed: Record<CallState, CallState[]> = {
    IDLE: ["RINGING","OFFHOOK"],
    RINGING: ["OFFHOOK","HUNGUP"],
    OFFHOOK: ["HUNGUP"],
    HUNGUP: ["IDLE"],
  }
  return (allowed[from] || []).includes(to)
}
export function redactNumber(n: string): string {
  const digits = n.replace(/[^0-9]/g,'')
  if(digits.length<=4) return "****"
  const last4 = digits.slice(-4)
  return n.trim().startsWith('+') ? `+** ****${last4}` : `** ****${last4}`
}
export function buildSmsSendPayload(address: string, body: string, subscriptionId?: number) {
  if(!isValidNumber(address)) throw new Error("invalid number")
  if(!isValidSmsBody(body)) throw new Error("invalid body")
  return { address, body, subscriptionId: subscriptionId ?? null }
}
export function buildCallStartPayload(number: string, subscriptionId?: number) {
  if(!isValidNumber(number)) throw new Error("invalid number")
  return { number, subscriptionId: subscriptionId ?? null }
}
