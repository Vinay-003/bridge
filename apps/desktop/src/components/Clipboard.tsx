import { useEffect, useState } from 'react'
import { bridge } from '../lib/bridge'

export function Clipboard() {
  const [text, setText] = useState("")
  const [remote, setRemote] = useState("")
  const [auto, setAuto] = useState(true)
  useEffect(()=>{
    const onSync = (m:any)=>{
      const src = m.payload.source || ""
      if(src==="desktop") return // ignore own echo
      const b64 = m.payload.data_b64 || ""
      try { setRemote(atob(b64)) } catch { setRemote(b64) }
      // auto-copy to system clipboard if enabled and from phone
      if(auto && b64 && src!=="desktop") {
        try { navigator.clipboard.writeText(atob(b64)) } catch {}
      }
    }
    bridge.on("clipboard.sync", onSync)
    // poll system clipboard for changes (if permission granted)
    let last = ""
    const iv = setInterval(async ()=>{
      if(!auto) return
      try {
        const t = await navigator.clipboard.readText()
        if(t && t!==last && t!==remote) { last=t; bridge.send("clipboard.sync",{mime:"text/plain", data_b64:btoa(t), ts:Date.now(), source:"desktop"}) }
      } catch {}
    }, 1500)
    return ()=> { bridge.off("clipboard.sync", onSync); clearInterval(iv); }
  },[auto, remote])
  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-white font-semibold">Clipboard</h3>
        <label className="text-xs text-bridge-muted flex items-center gap-2"><input type="checkbox" checked={auto} onChange={e=>setAuto(e.target.checked)} /> Auto-sync</label>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <div className="text-xs text-bridge-muted mb-2">Send to phone</div>
          <textarea value={text} onChange={e=>setText(e.target.value)} placeholder="Text, link or image b64..." className="w-full h-28 bg-[#0f0f14] border border-bridge-border rounded-xl p-3 text-sm text-white placeholder:bridge-muted focus:outline-none focus:border-bridge-accent" />
          <button onClick={()=>{
            if(!text) return
            const b64=btoa(text); bridge.send("clipboard.sync",{mime:"text/plain", data_b64:b64, ts:Date.now(), source:"desktop"})
            try{ navigator.clipboard.writeText(text)}catch{}
            setText("")
          }} className="mt-2 bg-bridge-accent text-white px-4 py-2 rounded-xl text-sm">Sync</button>
        </div>
        <div>
          <div className="text-xs text-bridge-muted mb-2">From phone</div>
          <div className="h-28 bg-[#0f0f14] border border-bridge-border rounded-xl p-3 text-sm text-white overflow-auto whitespace-pre-wrap">{remote || "No clipboard yet — copy on phone"}</div>
          <button onClick={()=>{
            if(remote) { navigator.clipboard.writeText(remote); }
          }} className="mt-2 text-xs border border-bridge-border text-white px-3 py-1.5 rounded-lg">Copy to system</button>
        </div>
      </div>
    </div>
  )
}
