import { describe, it, expect } from 'vitest'
import { isValidDeviceId, isValidStunServer, isOpaqueBlob, canTransitionRelay, validateRelayAnnouncePayload, validateRelayRelayPayload, encodeStunBindingRequest, rateLimitCheck, RELAY_URL, STUN_SERVER } from './relay'

describe('relay validation', () => {
  it('valid deviceId', () => {
    expect(isValidDeviceId('linux-abc-123')).toBe(true)
    expect(isValidDeviceId('phone_xyz')).toBe(true)
    expect(isValidDeviceId('')).toBe(false)
    expect(isValidDeviceId('a'.repeat(65))).toBe(false)
    expect(isValidDeviceId('bad/id')).toBe(false)
  })
  it('stun server', () => {
    expect(isValidStunServer('stun.l.google.com:19302')).toBe(true)
    expect(isValidStunServer('bad')).toBe(false)
    expect(isValidStunServer('host:99999')).toBe(false)
    expect(isValidStunServer('host:0')).toBe(false)
  })
  it('opaque blob', () => {
    const ok = btoa(String.fromCharCode(...new Uint8Array(64).fill(0x42)))
    expect(isOpaqueBlob(ok)).toBe(true)
    expect(isOpaqueBlob('{"plaintext":1}')).toBe(false)
    expect(isOpaqueBlob('short')).toBe(false)
    expect(isOpaqueBlob('')).toBe(false)
  })
  it('relay url + stun const', () => {
    expect(RELAY_URL).toBe('https://relay.bridge.dev/v1/announce')
    expect(STUN_SERVER).toBe('stun.l.google.com:19302')
  })
  it('state machine', () => {
    expect(canTransitionRelay('DISCONNECTED','ANNOUNCING')).toBe(true)
    expect(canTransitionRelay('ANNOUNCING','HOLE_PUNCHING')).toBe(true)
    expect(canTransitionRelay('HOLE_PUNCHING','CONNECTED_DIRECT')).toBe(true)
    expect(canTransitionRelay('HOLE_PUNCHING','RELAY_READY')).toBe(true)
    expect(canTransitionRelay('RELAY_READY','CONNECTED_VIA_RELAY')).toBe(true)
    expect(canTransitionRelay('DISCONNECTED','CONNECTED_DIRECT')).toBe(false)
    expect(canTransitionRelay('CONNECTED_DIRECT','ANNOUNCING')).toBe(false)
  })
  it('validate announce ok', () => {
    const blob = btoa(String.fromCharCode(...new Uint8Array(64).fill(0x42)))
    expect(validateRelayAnnouncePayload({deviceId:'linux-abc', blob, ts:Date.now(), fp:'aabbcc112233', mappedAddr:'1.2.3.4:5678', stunServer:'stun.l.google.com:19302', nonce:'aabbccdd'})).toBe(null)
  })
  it('validate announce invalid device', () => {
    const blob = btoa(String.fromCharCode(...new Uint8Array(64).fill(0x42)))
    expect(validateRelayAnnouncePayload({deviceId:'', blob})).not.toBe(null)
    expect(validateRelayAnnouncePayload({deviceId:'bad/id', blob})).not.toBe(null)
  })
  it('validate relay opque', () => {
    const blob = btoa(String.fromCharCode(...new Uint8Array(64).fill(0x42)))
    expect(validateRelayRelayPayload({to:'phone-xyz', from:'linux-abc', blob, ts:Date.now(), nonce:'11223344'})).toBe(null)
    expect(validateRelayRelayPayload({to:'', from:'linux-abc', blob})).not.toBe(null)
    expect(validateRelayRelayPayload({to:'phone-xyz', from:'linux-abc', blob:'{"plain":1}'})).not.toBe(null)
  })
  it('stun encode', () => {
    const txid = new Uint8Array(12).fill(1)
    const req = encodeStunBindingRequest(txid)
    expect(req.length).toBe(20)
    expect(req[0]).toBe(0); expect(req[1]).toBe(0x01)
    expect(req[4]).toBe(0x21); expect(req[5]).toBe(0x12)
  })
  it('rate limit', () => {
    const ts: number[] = []
    for(let i=0;i<20;i++) expect(rateLimitCheck(ts, 1000)).toBe(false)
    expect(rateLimitCheck(ts, 1000)).toBe(true)
    expect(rateLimitCheck(ts, 70000)).toBe(false)
  })
})
