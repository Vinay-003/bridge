import { useEffect, useState } from 'react'
import { bridge } from './lib/bridge'
import { useBridgeStore } from './lib/store'
import { Pairing } from './components/Pairing'
import { Status } from './components/Status'
import { Files } from './components/Files'
import { Clipboard } from './components/Clipboard'
import { Notifications } from './components/Notifications'
import { Media } from './components/Media'
import { Telephony } from './components/Telephony'

export default function App(){
  const connected = useBridgeStore(s=>s.connected)
  const [tab, setTab] = useState<"overview"|"files"|"media"|"telephony">("overview")
  useEffect(()=>{
    const handler = (s:"disconnected"|"connecting"|"connected")=> useBridgeStore.getState().set({connected: s==="connected"})
    bridge.onState = handler
    // if already connected before handler set, sync
    if(bridge.ws && bridge.ws.readyState===1) handler("connected")
    // request status immediately
    bridge.send("pairing.hello", { client: "desktop-app" })
  },[])
  return (
    <div className="min-h-screen bg-bridge-bg text-white">
      <header className="sticky top-0 z-10 backdrop-blur bg-bridge-bg/80 border-b border-bridge-border">
        <div className="max-w-6xl mx-auto px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-bridge-accent to-bridge-accent2 flex items-center justify-center font-black">B</div>
            <div>
              <div className="font-semibold tracking-tight">Bridge</div>
              <div className="text-xs text-bridge-muted">Linux ↔ Android continuity · LAN-first · E2E</div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <span className={`h-2 w-2 rounded-full ${connected?'bg-emerald-400 animate-pulse':'bg-zinc-600'}`} />
            <span className="text-xs text-bridge-muted">{connected?'WS 8443 connected':'disconnected — daemon needed'}</span>
            <button onClick={()=>{
              if(bridge.ws) bridge.ws.close()
              bridge.send("pairing.hello", {})
              setTimeout(()=>bridge.connect(), 500)
            }} className="text-xs border border-bridge-border text-white px-3 py-1 rounded-full">Reconnect</button>
            <nav className="ml-2 flex gap-1 bg-bridge-card border border-bridge-border rounded-full p-1">
              {(["overview","files","media","telephony"] as const).map(t=>(
                <button key={t} onClick={()=>setTab(t)} className={`px-4 py-1.5 rounded-full text-xs capitalize ${tab===t?'bg-white text-black':'text-bridge-muted'}`}>{t}</button>
              ))}
            </nav>
          </div>
        </div>
      </header>

      <main className="max-w-6xl mx-auto px-6 py-6 space-y-6">
        {tab==="overview" && (
          <>
            <Status />
            <div className="grid grid-cols-1 xl:grid-cols-12 gap-6">
              <div className="xl:col-span-7 space-y-6 min-w-0">
                <Pairing />
                <Clipboard />
              </div>
              <div className="xl:col-span-5 space-y-6 min-w-0">
                <Notifications />
              </div>
            </div>
            <Media />
          </>
        )}
        {tab==="files" && <div className="space-y-6"><Status /><Files /><Clipboard /></div>}
        {tab==="media" && <div className="space-y-6"><Media /><Status /></div>}
        {tab==="telephony" && <div className="space-y-6"><Status /><Telephony /></div>}
        <footer className="text-center text-xs text-bridge-muted py-6 border-t border-bridge-border mt-8">
          Bridge MVP · Tauri+Rust daemon on 8443 · mDNS _bridge._tcp · QUIC bulk · WebRTC media · v4l2loopback /dev/video10 · PipeWire Bridge Mic
        </footer>
      </main>
    </div>
  )
}
