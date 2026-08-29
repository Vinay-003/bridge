import { describe, it, expect } from 'vitest'
import { isValidPluginId, isValidPluginVersion, sanitizePluginPath, validatePluginManifest, validatePluginLoadPayload, canPluginAccess, canTransitionPlugin, mockTranslate, ALLOWED_CAPS } from './plugin'

describe('plugin validation', () => {
  it('valid pluginId', () => {
    expect(isValidPluginId('example-translate')).toBe(true)
    expect(isValidPluginId('ab')).toBe(false)
    expect(isValidPluginId('BadCaps')).toBe(false)
    expect(isValidPluginId('a'.repeat(33))).toBe(false)
  })
  it('valid version', () => {
    expect(isValidPluginVersion('0.1.0')).toBe(true)
    expect(isValidPluginVersion('1.2.3')).toBe(true)
    expect(isValidPluginVersion('0.1')).toBe(false)
    expect(isValidPluginVersion('01.0.0')).toBe(false)
  })
  it('sanitize plugin path', () => {
    expect(sanitizePluginPath('index.js')).toBe(null)
    expect(sanitizePluginPath('src/main.wasm')).toBe(null)
    expect(sanitizePluginPath('../escape.js')).not.toBe(null)
    expect(sanitizePluginPath('/abs.js')).not.toBe(null)
    expect(sanitizePluginPath('evil.txt')).not.toBe(null)
  })
  it('validate manifest ok', () => {
    expect(validatePluginManifest({name:'example-translate', version:'0.1.0', entry:'index.js', capabilities:['notify','clipboard']})).toBe(null)
  })
  it('validate manifest bad caps', () => {
    expect(validatePluginManifest({name:'bad', version:'0.1.0', entry:'index.js', capabilities:['evil']})).not.toBe(null)
  })
  it('validate manifest traversal', () => {
    expect(validatePluginManifest({name:'bad', version:'0.1.0', entry:'../../etc/passwd', capabilities:['notify']})).not.toBe(null)
  })
  it('validate load payload', () => {
    expect(validatePluginLoadPayload({pluginId:'example-translate'})).toBe(null)
    expect(validatePluginLoadPayload({pluginId:'bad/cap'})).not.toBe(null)
    expect(validatePluginLoadPayload({})).not.toBe(null)
  })
  it('can access', () => {
    expect(canPluginAccess(['notify','clipboard'],'notify')).toBe(true)
    expect(canPluginAccess(['notify'],'storage')).toBe(false)
  })
  it('state transitions', () => {
    expect(canTransitionPlugin('UNLOADED','LOADING')).toBe(true)
    expect(canTransitionPlugin('LOADING','LOADED')).toBe(true)
    expect(canTransitionPlugin('LOADED','RUNNING')).toBe(true)
    expect(canTransitionPlugin('RUNNING','RELOADING')).toBe(true)
    expect(canTransitionPlugin('RELOADING','RUNNING')).toBe(true)
    expect(canTransitionPlugin('UNLOADED','RUNNING')).toBe(false)
    expect(canTransitionPlugin('RUNNING','LOADED')).toBe(false)
  })
  it('mock translate', () => {
    expect(mockTranslate('hello','en')).toBe('olleh [translated:en]')
    expect(mockTranslate('Bonjour')).toContain('[translated')
  })
  it('allowed caps', () => {
    expect(ALLOWED_CAPS).toContain('notify')
    expect(ALLOWED_CAPS).toContain('clipboard')
    expect(ALLOWED_CAPS).toContain('storage')
  })
})
