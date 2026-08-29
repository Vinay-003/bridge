import { describe, it, expect } from 'vitest'
import { isValidNumber, isValidSmsBody, canTransition, redactNumber, buildSmsSendPayload, buildCallStartPayload } from './telephony'
import { BridgeClient } from './bridge'

describe('telephony validation', () => {
  it('valid numbers pass', () => {
    expect(isValidNumber('+33612345678')).toBe(true)
    expect(isValidNumber('0612345678')).toBe(true)
    expect(isValidNumber('+1 650 555 1234')).toBe(true)
    expect(isValidNumber('(650) 555-1234')).toBe(true)
  })
  it('invalid numbers fail', () => {
    expect(isValidNumber('123')).toBe(false)
    expect(isValidNumber('')).toBe(false)
    expect(isValidNumber('abc')).toBe(false)
    expect(isValidNumber('+33-6-12-34-56-7890123456')).toBe(false)
  })
  it('sms body validation', () => {
    expect(isValidSmsBody('Hello')).toBe(true)
    expect(isValidSmsBody('')).toBe(false)
    expect(isValidSmsBody('a'.repeat(919))).toBe(false)
    expect(isValidSmsBody('a'.repeat(918))).toBe(true)
  })
  it('call state machine', () => {
    expect(canTransition('IDLE','RINGING')).toBe(true)
    expect(canTransition('RINGING','OFFHOOK')).toBe(true)
    expect(canTransition('RINGING','HUNGUP')).toBe(true)
    expect(canTransition('OFFHOOK','HUNGUP')).toBe(true)
    expect(canTransition('HUNGUP','IDLE')).toBe(true)
    expect(canTransition('IDLE','HUNGUP')).toBe(false)
    expect(canTransition('OFFHOOK','RINGING')).toBe(false)
  })
  it('redact number', () => {
    expect(redactNumber('+33612345678')).toBe('+** ****5678')
    expect(redactNumber('0612345678')).toBe('** ****5678')
    expect(redactNumber('123')).toBe('****')
  })
  it('build payloads validate', () => {
    expect(buildSmsSendPayload('+33612345678','Hi').address).toBe('+33612345678')
    expect(()=>buildSmsSendPayload('bad','Hi')).toThrow()
    expect(()=>buildSmsSendPayload('+33612345678','')).toThrow()
    expect(buildCallStartPayload('+33612345678').number).toBe('+33612345678')
    expect(()=>buildCallStartPayload('bad')).toThrow()
  })
  it('bridge message types for telephony roundtrip via client send', () => {
    const c = new BridgeClient('ws://localhost:8443')
    const m = c.send('sms.send', { address: '+33612345678', body: 'Hello via Bridge' })
    expect(m.type).toBe('sms.send')
    expect(m.payload.address).toBe('+33612345678')
    const call = c.send('call.start', { number: '+33612345678' })
    expect(call.type).toBe('call.start')
    const list = c.send('sms.list', { limit: 50 })
    expect(list.type).toBe('sms.list')
    const audio = c.send('call.audio', { callId: 'uuid', sdp: 'v=0' })
    expect(audio.type).toBe('call.audio')
    const log = c.send('call.log', {})
    expect(log.type).toBe('call.log')
  })
})
