import { describe, it, expect } from 'vitest'
import { BridgeClient } from './bridge'

describe('BridgeClient', () => {
  it('creates message with required fields', () => {
    const c = new BridgeClient('ws://localhost:8443')
    const m = c.send('heartbeat.ping', { ok: true })
    expect(m.v).toBe(1)
    expect(m.type).toBe('heartbeat.ping')
    expect(m.id).toBeDefined()
    expect(m.payload.ok).toBe(true)
  })
  it('handles multiple handlers', () => {
    const c = new BridgeClient('ws://localhost:8443')
    let got = 0
    c.on('clipboard.sync', () => got++)
    c.on('clipboard.sync', () => got++)
    // simulate message
    const handlers = (c as any).handlers.get('clipboard.sync')
    expect(handlers.length).toBe(2)
  })
})
