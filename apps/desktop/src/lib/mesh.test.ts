import { describe, it, expect } from 'vitest'
import { canTransitionMesh, dominatesVector, isVectorConcurrent, mergeVector, isVectorMonotonic, lwwMerge, validateMeshSyncPayload, validateMeshConflictPayload } from './mesh'

describe('mesh state', () => {
  it('transitions', () => {
    expect(canTransitionMesh('IDLE','SYNCING')).toBe(true)
    expect(canTransitionMesh('SYNCING','CONFLICT')).toBe(true)
    expect(canTransitionMesh('CONFLICT','SYNCING')).toBe(true)
    expect(canTransitionMesh('SYNCING','IDLE')).toBe(true)
    expect(canTransitionMesh('CONFLICT','IDLE')).toBe(true)
    expect(canTransitionMesh('IDLE','CONFLICT')).toBe(false)
    expect(canTransitionMesh('IDLE','IDLE')).toBe(false)
  })
  it('vector dominates', () => {
    expect(dominatesVector({a:3,b:2},{a:2,b:2})).toBe(true)
    expect(dominatesVector({a:2,b:2},{a:3,b:2})).toBe(false)
    expect(dominatesVector({a:1},{a:1})).toBe(false)
  })
  it('concurrent', () => {
    expect(isVectorConcurrent({a:3,b:1},{a:2,b:2})).toBe(true)
    expect(isVectorConcurrent({a:3,b:2},{a:2,b:2})).toBe(false)
    expect(isVectorConcurrent({a:3},{a:3})).toBe(false)
  })
  it('merge', () => {
    expect(mergeVector({a:1},{b:2})).toEqual({a:1,b:2})
    expect(mergeVector({a:3,b:1},{a:2,b:5})).toEqual({a:3,b:5})
  })
  it('monotonic', () => {
    expect(isVectorMonotonic({a:2},{a:3},'a')).toBe(true)
    expect(isVectorMonotonic({a:2},{a:5},'a')).toBe(false)
    expect(isVectorMonotonic({a:2},{a:2},'a')).toBe(true)
  })
  it('lww', () => {
    const a = {text:'hello', mime:'text/plain', ts:1000, device_id:'a'}
    const b = {text:'world', mime:'text/plain', ts:2000, device_id:'b'}
    expect(lwwMerge(a,b).text).toBe('world')
    expect(lwwMerge(b,a).text).toBe('world')
    const c = {text:'aaa', mime:'text/plain', ts:2000, device_id:'a'}
    const d = {text:'bbb', mime:'text/plain', ts:2000, device_id:'b'}
    expect(lwwMerge(c,d).device_id).toBe('b')
  })
  it('validate mesh sync ok', () => {
    expect(validateMeshSyncPayload({deviceId:'phone-xyz', vectors:{'phone-xyz':1}, entries:[{path:'/report.pdf', vector:{'phone-xyz':1}}], ts: Date.now()})).toBe(null)
  })
  it('validate mesh sync missing device', () => {
    expect(validateMeshSyncPayload({vectors:{}, entries:[]})).not.toBe(null)
  })
  it('validate mesh sync invalid path', () => {
    expect(validateMeshSyncPayload({deviceId:'phone-xyz', vectors:{}, entries:[{path:'../bad'}]})).not.toBe(null)
  })
  it('validate mesh conflict ok', () => {
    expect(validateMeshConflictPayload({path:'/report.pdf', resolution:'lww', winner:'local'})).toBe(null)
    expect(validateMeshConflictPayload({path:'/report.pdf', resolution:'rename', winner:'remote', loserRename:'/report.pdf.conflict'})).toBe(null)
  })
  it('validate mesh conflict invalid', () => {
    expect(validateMeshConflictPayload({path:'/a', resolution:'bad', winner:'local'})).not.toBe(null)
    expect(validateMeshConflictPayload({path:'../a', resolution:'lww', winner:'local'})).not.toBe(null)
  })
})
