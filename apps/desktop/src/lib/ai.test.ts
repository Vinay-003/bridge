import { describe, it, expect } from 'vitest'
import { canTransitionAi, validateAiSummarizePayload, validateAiTranscribePayload, validateAiResultPayload, shouldRateLimit, localSummarize } from './ai'

describe('ai validation', () => {
  it('state transitions', () => {
    expect(canTransitionAi('IDLE','QUEUED')).toBe(true)
    expect(canTransitionAi('QUEUED','LOCAL')).toBe(true)
    expect(canTransitionAi('QUEUED','CLOUD')).toBe(true)
    expect(canTransitionAi('LOCAL','DONE')).toBe(true)
    expect(canTransitionAi('LOCAL','CLOUD')).toBe(true)
    expect(canTransitionAi('CLOUD','DONE')).toBe(true)
    expect(canTransitionAi('IDLE','DONE')).toBe(false)
    expect(canTransitionAi('DONE','QUEUED')).toBe(false)
    expect(canTransitionAi('DONE','IDLE')).toBe(true)
  })
  it('validate summarize ok', () => {
    expect(validateAiSummarizePayload({notifications:[{app:'WhatsApp', body:'hello'}], maxLen:200})).toBe(null)
  })
  it('validate summarize empty', () => {
    expect(validateAiSummarizePayload({notifications:[], maxLen:200})).not.toBe(null)
  })
  it('validate summarize too many', () => {
    const many = Array(21).fill({app:'A', body:'hi'})
    expect(validateAiSummarizePayload({notifications:many})).not.toBe(null)
  })
  it('validate summarize body too long', () => {
    expect(validateAiSummarizePayload({notifications:[{app:'A', body:'a'.repeat(501)}]})).not.toBe(null)
  })
  it('validate transcribe ok', () => {
    const b64 = btoa(String.fromCharCode(...new Uint8Array(100).fill(0x42)))
    expect(validateAiTranscribePayload({audio_b64:b64, format:'opus', lang:'en'})).toBe(null)
    expect(validateAiTranscribePayload({audio_b64:b64, format:'wav'})).toBe(null)
  })
  it('validate transcribe invalid format', () => {
    const b64 = btoa('hi')
    expect(validateAiTranscribePayload({audio_b64:b64, format:'evil'})).not.toBe(null)
  })
  it('validate transcribe empty', () => {
    expect(validateAiTranscribePayload({audio_b64:'', format:'opus'})).not.toBe(null)
  })
  it('validate result ok', () => {
    expect(validateAiResultPayload({kind:'summarize', text:'hi', model:'llama.cpp-local'})).toBe(null)
    expect(validateAiResultPayload({kind:'transcribe', text:'hi', model:'whisper.cpp-local'})).toBe(null)
  })
  it('validate result invalid kind', () => {
    expect(validateAiResultPayload({kind:'evil', text:'hi', model:'m'})).not.toBe(null)
  })
  it('rate limit', () => {
    const ts: number[] = []
    for(let i=0;i<10;i++) expect(shouldRateLimit(ts, 1000)).toBe(false)
    expect(shouldRateLimit(ts, 1000)).toBe(true)
    expect(shouldRateLimit(ts, 70000)).toBe(false)
  })
  it('local summarize', () => {
    const notifs = [{app:'WhatsApp', body:'hello'}, {app:'WhatsApp', body:'hi'}, {app:'Gmail', body:'test'}]
    const s = localSummarize(notifs, 200)
    expect(s).toContain('3 notifications')
    expect(s).toContain('WhatsApp')
    expect(s.length).toBeLessThanOrEqual(200)
  })
})
