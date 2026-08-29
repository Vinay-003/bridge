import { useEffect, useState } from 'react'
import { bridge } from '../lib/bridge'
import { validateAiSummarizePayload, validateAiTranscribePayload } from '../lib/ai'

type PluginInfo = {
  id: string
  name: string
  version: string
  displayName: string
  description: string
  entry: string
  capabilities: string[]
  state: string
}

type AiResult = {
  requestId: string
  kind: string
  text: string
  model: string
  tokens?: number
  durationMs?: number
}

export function Plugins(){
  const [plugins, setPlugins] = useState<PluginInfo[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string|null>(null)
  const [aiResults, setAiResults] = useState<AiResult[]>([])
  const [summarizeInput, setSummarizeInput] = useState('{"notifications":[{"app":"WhatsApp","title":"Mom","body":"Call me"}],"maxLen":200}')
  const [transcribeInput, setTranscribeInput] = useState('{"audio_b64":"'+btoa('fake audio bytes for 30s')+'","format":"opus","lang":"en","cloudConsent":true}')
  const [relayEnabled, setRelayEnabled] = useState(false)

  const refresh = () => {
    setLoading(true)
    bridge.send("plugin.list", {})
  }

  useEffect(()=>{
    const onList = (m: any) => {
      if (m.type === "plugin.list") {
        setPlugins(m.payload.plugins || [])
        setLoading(false)
      }
    }
    const onLoad = (m: any) => {
      if (m.type === "plugin.load") {
        if (m.payload.ok) refresh()
        else setError(m.payload.error)
      }
    }
    const onEmit = (m: any) => {
      if (m.type === "plugin.emit") {
        if (m.payload.error) setError(m.payload.error)
      }
    }
    const onAiResult = (m: any) => {
      if (m.type === "ai.result") {
        setAiResults(prev=> [m.payload as AiResult, ...prev].slice(0,5))
      }
    }
    const onError = (m:any) => {
      if (m.type==="error") {
        const code = m.payload.code
        if (["capability_denied","validation","rate_limited","cloud_consent_required","ai_unavailable","plugin_not_found"].includes(code)) {
          setError(`${code}: ${m.payload.message||m.payload.error}`)
        }
      }
    }
    bridge.on("plugin.list", onList)
    bridge.on("plugin.load", onLoad)
    bridge.on("plugin.emit", onEmit)
    bridge.on("ai.result", onAiResult)
    bridge.on("error", onError)
    refresh()
    return ()=>{
      bridge.off("plugin.list", onList)
      bridge.off("plugin.load", onLoad)
      bridge.off("plugin.emit", onEmit)
      bridge.off("ai.result", onAiResult)
      bridge.off("error", onError)
    }
  }, [])

  const doLoad = (id: string) => {
    bridge.send("plugin.load", {pluginId: id})
  }
  const doPluginEmitTest = (pluginId: string, event: string) => {
    bridge.send("plugin.emit", {pluginId, event, data:{test:true}})
  }

  const doSummarize = () => {
    try {
      const payload = JSON.parse(summarizeInput)
      // local validation before send
      const err = validateAiSummarizePayload(payload)
      if (err) { setError(err); return}
      payload.requestId = crypto.randomUUID()
      payload.cloudConsent = true // for demo, allow cloud fallback
      bridge.send("ai.summarize", payload)
    } catch (e:any) { setError(e.message)}
  }
  const doTranscribe = () => {
    try {
      const payload = JSON.parse(transcribeInput)
      const err = validateAiTranscribePayload(payload)
      if (err) { setError(err); return}
      payload.requestId = crypto.randomUUID()
      payload.cloudConsent = true
      bridge.send("ai.transcribe", payload)
    } catch (e:any) { setError(e.message)}
  }

  const toggleRelay = () => {
    const newVal = !relayEnabled
    setRelayEnabled(newVal)
    // In real daemon, --relay flag enables E2E relay; here we send relay.announce mock
    if (newVal) {
      const blob = btoa(String.fromCharCode(...new Uint8Array(64).fill(0x42)))
      bridge.send("relay.announce", {deviceId:"desktop-"+Math.random().toString(36).slice(2,6), blob, ts:Date.now(), fp:"aabbcc112233", mappedAddr:"1.2.3.4:5678", stunServer:"stun.l.google.com:19302", nonce: Math.random().toString(16).slice(2,10)})
    }
  }

  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-white font-semibold">Plugin Platform + AI</h3>
        <span className="text-xs text-bridge-muted">wasmtime/deno sandbox stub • hot reload • capability check</span>
      </div>

      {error && <div className="text-xs text-amber-400 bg-amber-950/30 border border-amber-800 rounded-xl p-2">{error} <button onClick={()=> setError(null)} className="ml-2 underline">dismiss</button></div>}

      <div className="flex items-center gap-2">
        <button onClick={refresh} className="px-3 py-1 rounded-full border border-bridge-border text-bridge-muted text-xs">{loading?"loading…":"↻ Refresh plugins"}</button>
        <span className="text-xs text-bridge-muted">{plugins.length} plugins</span>
        <button onClick={toggleRelay} className={`px-3 py-1 rounded-full text-xs ${relayEnabled?'bg-bridge-accent text-black':'border border-bridge-border text-bridge-muted'}`}>{relayEnabled?"Relay ON (E2E opaque)":"Enable Relay (--relay)"}</button>
        <span className="text-xs text-bridge-muted">STUN {relayEnabled?"stun.l.google.com:19302":"disabled"}</span>
      </div>

      <div className="border border-bridge-border rounded-2xl overflow-hidden">
        <div className="grid grid-cols-12 gap-2 px-4 py-2 bg-[#0f0f14] text-xs text-bridge-muted border-b border-bridge-border">
          <span className="col-span-3">Plugin</span><span className="col-span-2">Version</span><span className="col-span-3">Capabilities</span><span className="col-span-2">State</span><span className="col-span-2">Actions</span>
        </div>
        {plugins.length===0 && <div className="px-4 py-6 text-center text-sm text-bridge-muted">No plugins — example-translate at plugins/example-translate/ (mock)</div>}
        {plugins.map(p=> (
          <div key={p.id} className="grid grid-cols-12 gap-2 px-4 py-2 items-center hover:bg-white/[0.04] border-b border-bridge-border/50">
            <span className="col-span-3 text-sm text-white">{p.displayName||p.name}<span className="text-xs text-bridge-muted block">{p.id}</span></span>
            <span className="col-span-2 text-xs text-bridge-muted">{p.version}</span>
            <span className="col-span-3 text-xs text-bridge-muted">{p.capabilities.join(', ')}</span>
            <span className={`col-span-2 text-xs px-2 py-1 rounded-full border text-center ${p.state==="RUNNING"?"border-emerald-500 text-emerald-400":"border-bridge-border text-bridge-muted"}`}>{p.state}</span>
            <span className="col-span-2 flex gap-1">
              <button onClick={()=> doLoad(p.id)} className="text-xs px-2 py-1 rounded-full border border-bridge-border text-white">Reload</button>
              <button onClick={()=> doPluginEmitTest(p.id, "notify.new")} className="text-xs px-2 py-1 rounded-full border border-bridge-border text-bridge-muted">Test notify</button>
            </span>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div className="space-y-2 bg-[#0f0f14] border border-bridge-border rounded-2xl p-4">
          <h4 className="text-white text-sm font-semibold">AI Summarize (ai.summarize)</h4>
          <p className="text-xs text-bridge-muted">Local llama.cpp if available else cloud fallback (rate limited 10/min). Mock: counts per app.</p>
          <textarea value={summarizeInput} onChange={e=> setSummarizeInput(e.target.value)} rows={4} className="w-full bg-bridge-card border border-bridge-border rounded-xl p-2 text-xs text-white font-mono"/>
          <button onClick={doSummarize} className="px-4 py-1.5 rounded-full bg-bridge-accent text-black text-xs font-semibold">Summarize → ai.result</button>
          <div className="text-xs text-bridge-muted">Example plugin translate uses same AI: bridge.on('notify.new') → mockTranslate + clipboard</div>
        </div>
        <div className="space-y-2 bg-[#0f0f14] border border-bridge-border rounded-2xl p-4">
          <h4 className="text-white text-sm font-semibold">AI Transcribe (ai.transcribe)</h4>
          <p className="text-xs text-bridge-muted">Local whisper.cpp if available else cloud (opus/wav 5MB, rate 10/min). Mock.</p>
          <textarea value={transcribeInput} onChange={e=> setTranscribeInput(e.target.value)} rows={4} className="w-full bg-bridge-card border border-bridge-border rounded-xl p-2 text-xs text-white font-mono"/>
          <button onClick={doTranscribe} className="px-4 py-1.5 rounded-full bg-bridge-accent text-black text-xs font-semibold">Transcribe → ai.result</button>
          <div className="text-xs text-bridge-muted">Android ai/ stub: on-device NNAPI else cloud.</div>
        </div>
      </div>

      {aiResults.length>0 && (
        <div className="space-y-2">
          <h4 className="text-white text-sm">AI Results (ai.result)</h4>
          {aiResults.map((r,i)=>(
            <div key={i} className="bg-[#0f0f14] border border-bridge-border rounded-xl p-3">
              <div className="text-xs text-bridge-muted">{r.kind} via {r.model} {r.tokens?`· ${r.tokens} tokens`:''} {r.durationMs?`· ${r.durationMs}ms`:''}</div>
              <div className="text-sm text-white">{r.text}</div>
              <div className="text-xs text-bridge-muted">req {r.requestId}</div>
            </div>
          ))}
        </div>
      )}

      <div className="text-xs text-bridge-muted">Capabilities: notify→ onNotify, clipboard→ bridge.clipboard, storage→ bridge.storage (capped), ai.*→ local whisper/llama or cloud with consent. Hot reload via notify watch 500ms. Fuel 10M (wasmtime stub). E2E relay opaque via Noise, STUN fallback stun.l.google.com:19302.</div>
    </div>
  )
}
