import { describe, it, expect } from 'vitest'
import { useBridgeStore } from './store'
describe('store', () => {
  it('holds clipboard and notifs', () => {
    const s = useBridgeStore.getState()
    expect(s.notifs).toEqual([])
    useBridgeStore.getState().set({ notifs: [{key:'k',app:'WhatsApp',title:'hi',body:'b',ts:0,hasReply:true}] })
    expect(useBridgeStore.getState().notifs.length).toBe(1)
  })
})
