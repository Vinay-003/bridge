import { describe, it, expect } from 'vitest'
import { isValidStoragePath, sanitizeStoragePath, canTransitionStorage, isVectorConcurrent, dominatesVector, chunkStorageFile, parseTrashInfo, formatTrashPath } from './storage'
import { BridgeClient } from './bridge'

describe('storage path validation', () => {
  it('valid paths', () => {
    expect(isValidStoragePath('/')).toBe(true)
    expect(isValidStoragePath('/Photos')).toBe(true)
    expect(isValidStoragePath('/report.pdf')).toBe(true)
    expect(isValidStoragePath('/a/b/c')).toBe(true)
    expect(isValidStoragePath('Photos/img.jpg')).toBe(true)
  })
  it('rejects traversal', () => {
    expect(isValidStoragePath('../secret')).toBe(false)
    expect(isValidStoragePath('/a/../../etc/passwd')).toBe(false)
    expect(isValidStoragePath('/a/../b')).toBe(false)
    expect(isValidStoragePath('')).toBe(false)
    expect(isValidStoragePath('/\0bad')).toBe(false)
    expect(isValidStoragePath('/a/'.repeat(2000))).toBe(false)
  })
  it('sanitize', () => {
    expect(sanitizeStoragePath('/Photos/img.jpg')).toBe('Photos/img.jpg')
    expect(sanitizeStoragePath('/')).toBe('')
    expect(sanitizeStoragePath('a//b///c')).toBe('a/b/c')
    expect(() => sanitizeStoragePath('../escape')).toThrow()
    expect(() => sanitizeStoragePath('')).toThrow()
  })
})

describe('storage state machine', () => {
  it('valid transitions IDLE->SCANNING->SYNCING->DONE', () => {
    expect(canTransitionStorage('IDLE','SCANNING')).toBe(true)
    expect(canTransitionStorage('SCANNING','SYNCING')).toBe(true)
    expect(canTransitionStorage('SYNCING','DONE')).toBe(true)
    expect(canTransitionStorage('DONE','IDLE')).toBe(true)
  })
  it('conflict flow', () => {
    expect(canTransitionStorage('SYNCING','CONFLICT')).toBe(true)
    expect(canTransitionStorage('CONFLICT','SYNCING')).toBe(true)
    expect(canTransitionStorage('SCANNING','DONE')).toBe(true)
  })
  it('invalid', () => {
    expect(canTransitionStorage('IDLE','CONFLICT')).toBe(false)
    expect(canTransitionStorage('IDLE','SYNCING')).toBe(false)
    expect(canTransitionStorage('DONE','SCANNING')).toBe(false)
    expect(canTransitionStorage('CONFLICT','DONE')).toBe(false)
  })
})

describe('vector clock', () => {
  it('dominates', () => {
    expect(dominatesVector({daemon:3,phone:2},{daemon:2,phone:2})).toBe(true)
    expect(dominatesVector({daemon:2,phone:2},{daemon:3,phone:2})).toBe(false)
    expect(dominatesVector({daemon:1},{daemon:1})).toBe(false) // equal not dominates
  })
  it('concurrent', () => {
    expect(isVectorConcurrent({daemon:3,phone:1},{daemon:2,phone:2})).toBe(true)
    expect(isVectorConcurrent({daemon:3,phone:2},{daemon:2,phone:2})).toBe(false)
    expect(isVectorConcurrent({daemon:3},{daemon:3})).toBe(false)
  })
})

describe('chunk storage file', () => {
  it('slices 2.5MB file into 3 chunks 1MB SHA256', async () => {
    const data = new Uint8Array(2.5 * 1024 * 1024)
    for(let i=0;i<data.length;i++) data[i]= i % 256
    const chunks = await chunkStorageFile('test-id','big.bin', data.buffer)
    expect(chunks.length).toBe(3)
    expect(chunks[0].offset).toBe(0)
    expect(chunks[1].offset).toBe(1048576)
    expect(chunks[2].offset).toBe(2097152)
    expect(chunks[0].total).toBe(3)
    expect(chunks[0].sha256).toMatch(/^[0-9a-f]{64}$/)
    // 4GB offset check: chunk offset is u64
    expect(chunks[2].size).toBe(data.length)
  })
  it('handles empty not valid but still chunks', async () => {
    const data = new Uint8Array(0)
    const chunks = await chunkStorageFile('id','empty', data.buffer)
    expect(chunks.length).toBe(0)
  })
})

describe('trash info', () => {
  it('parse and format', () => {
    const info = `[Trash Info]\nPath=/home/user/Bridge/old.pdf\nDeletionDate=2026-08-13T12:00:00Z\n`
    const parsed = parseTrashInfo(info)
    expect(parsed.Path).toBe('/home/user/Bridge/old.pdf')
    expect(parsed.DeletionDate).toBe('2026-08-13T12:00:00Z')
    const formatted = formatTrashPath('/home/user/Bridge/old.pdf','2026-08-13T12:00:00Z')
    expect(formatted).toContain('Path=/home/user/Bridge/old.pdf')
  })
})

describe('bridge message types for storage roundtrip', () => {
  it('storage.* types', () => {
    const c = new BridgeClient('ws://localhost:8443')
    expect(c.send('storage.ls', {path:'/' }).type).toBe('storage.ls')
    expect(c.send('storage.stat', {path:'/a'}).type).toBe('storage.stat')
    expect(c.send('storage.mkdir', {path:'/new'}).type).toBe('storage.mkdir')
    expect(c.send('storage.rm', {path:'/old', toTrash:true}).type).toBe('storage.rm')
    expect(c.send('storage.sync', {id:'u', path:'/a', size:1, offset:0, total:1, index:0, sha256:'a'.repeat(64), data_b64:''}).type).toBe('storage.sync')
    expect(c.send('storage.conflict', {path:'/a', resolution:'lww'}).type).toBe('storage.conflict')
  })
})
