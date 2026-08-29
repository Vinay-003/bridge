import { describe, it, expect } from 'vitest'
import { isValidAction, clamp01, normToPx, pxToNorm, canTransition, validateInputEvent, buildInputEventPayload, shouldThrottle, coalesceMove, canvasToNorm, rateLimitCheck } from './control'
import { BridgeClient } from './bridge'

describe('control validation', () => {
  it('valid actions pass', () => {
    expect(isValidAction('tap')).toBe(true)
    expect(isValidAction('move')).toBe(true)
    expect(isValidAction('pinch')).toBe(true)
    expect(isValidAction('home')).toBe(true)
    expect(isValidAction('back')).toBe(true)
    expect(isValidAction('evil')).toBe(false)
  })
  it('clamp 0..1', () => {
    expect(clamp01(0.5)).toBe(0.5)
    expect(clamp01(-0.1)).toBe(null)
    expect(clamp01(1.5)).toBe(null)
    expect(clamp01(NaN)).toBe(null)
    expect(clamp01(0)).toBe(0)
    expect(clamp01(1)).toBe(1)
  })
  it('normToPx', () => {
    expect(normToPx(0.42, 1080)).toBe(453)
    expect(normToPx(0.5, 200)).toBe(100)
    expect(pxToNorm(453, 1080)).toBeCloseTo(0.419)
  })
  it('control state machine', () => {
    expect(canTransition('DISABLED','ENABLED')).toBe(true)
    expect(canTransition('ENABLED','CONTROLLING')).toBe(true)
    expect(canTransition('CONTROLLING','PAUSED')).toBe(true)
    expect(canTransition('PAUSED','ENABLED')).toBe(true)
    expect(canTransition('CONTROLLING','ENABLED')).toBe(true)
    expect(canTransition('PAUSED','DISABLED')).toBe(true)
    expect(canTransition('ENABLED','DISABLED')).toBe(true)
    expect(canTransition('CONTROLLING','DISABLED')).toBe(true)
    expect(canTransition('DISABLED','CONTROLLING')).toBe(false)
    expect(canTransition('ENABLED','PAUSED')).toBe(false)
    expect(canTransition('PAUSED','CONTROLLING')).toBe(false)
  })
  it('validate input event ok', () => {
    expect(validateInputEvent({x:0.42,y:0.71,action:"tap"})).toBe(null)
    expect(validateInputEvent({x:0.5,y:0.5,action:"move",displayId:0})).toBe(null)
    expect(validateInputEvent({action:"home"})).toBe(null)
    expect(validateInputEvent({action:"back"})).toBe(null)
    expect(validateInputEvent({action:"key",keyCode:4})).toBe(null)
  })
  it('validate input event invalid', () => {
    expect(validateInputEvent({x:1.5,y:0.5,action:"tap"})).not.toBe(null)
    expect(validateInputEvent({x:0.5,y:0.5,action:"evil" as any})).not.toBe(null)
    expect(validateInputEvent({x:0.5,y:0.5,action:"key" as any})).not.toBe(null)
    expect(validateInputEvent({x:0.5,y:0.5,action:"pinch",scale:10})).not.toBe(null)
    expect(validateInputEvent({x:0.5,y:0.5,action:"tap",pressure:2})).not.toBe(null)
  })
  it('build payload throws', () => {
    expect(buildInputEventPayload({x:0.5,y:0.5,action:"tap"}).action).toBe("tap")
    expect(()=>buildInputEventPayload({x:1.5,y:0.5,action:"tap"})).toThrow()
    expect(()=>buildInputEventPayload({x:0.5,y:0.5,action:"evil" as any})).toThrow()
  })
  it('throttle', () => {
    expect(shouldThrottle(null, 1000)).toBe(false)
    expect(shouldThrottle(1000, 1005)).toBe(true)
    expect(shouldThrottle(1000, 1020)).toBe(false)
  })
  it('coalesce', () => {
    expect(coalesceMove({x:0.1,y:0.1,action:"move"}, {x:0.11,y:0.11,action:"move"}).x).toBe(0.11)
    expect(coalesceMove(null, {x:0.5,y:0.5,action:"tap"}).action).toBe("tap")
  })
  it('canvas scaling', () => {
    const rect = {left:0,top:0,width:800,height:600}
    const display = {width:1080,height:2400}
    const n = canvasToNorm(400,300,rect,display,false)
    expect(n.x).toBeCloseTo(0.5)
    expect(n.y).toBeCloseTo(0.5)
    const n2 = canvasToNorm(0,0,rect,display,false)
    expect(n2.x).toBe(0)
    expect(n2.y).toBe(0)
  })
  it('rate limit', () => {
    const ts: number[] = []
    for(let i=0;i<120;i++) expect(rateLimitCheck(ts, 1000)).toBe(false)
    expect(rateLimitCheck(ts, 1000)).toBe(true)
    expect(rateLimitCheck(ts, 2500)).toBe(false)
  })
  it('bridge message types for control roundtrip', () => {
    const c = new BridgeClient('ws://localhost:8443')
    const m = c.send('input.event', {x:0.5,y:0.5,action:"tap"})
    expect(m.type).toBe('input.event')
    const ack = c.send('input.ack', {ok:true})
    expect(ack.type).toBe('input.ack')
    const info = c.send('display.info', {displayId:0})
    expect(info.type).toBe('display.info')
    const frame = c.send('display.frame', {displayId:0,frame_b64:"abc"})
    expect(frame.type).toBe('display.frame')
    const start = c.send('control.start', {displayId:0})
    expect(start.type).toBe('control.start')
    const stop = c.send('control.stop', {displayId:0})
    expect(stop.type).toBe('control.stop')
  })
})
