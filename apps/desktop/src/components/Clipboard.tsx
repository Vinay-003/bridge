import { useEffect, useState } from 'react'
import { bridge } from '../lib/bridge'

export function Clipboard() {
  const [text, setText] = useState("")
  const [remote, setRemote] = useState("")
  useEffect(()=>{
    bridge.on("clipboard.sync", (m)=>{
      const decoded = (()=>{ try{ return atob(m.payload.data_b64)} catch{ return m.payload.data_b64}})();
      setRemote(decoded)
    })
  },[])
  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5">
      <h3 className="text-white font-semibold mb-3">Clipboard</h3>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <div className="text-xs text-bridge-muted mb-2">Send to phone</div>
          <textarea value={text} onChange={e=>setText(e.target.value)} placeholder="Text, link or image b64..." className="w-full h-28 bg-[#0f0f14] border border-bridge-border rounded-xl p-3 text-sm text-white placeholder:bridge-muted focus:outline-none focus:border-bridge-accent" />
          <button onClick={()=>{
            const b64=btoa(text); bridge.send("clipboard.sync",{mime:"text/plain", data_b64:b64, ts:Date.now(), source:"desktop"}) ; setText("")
          }} className="mt-2 bg-bridge-accent text-white px-4 py-2 rounded-xl text-sm">Sync</button>
        </div>
        <div>
          <div className="text-xs text-bridge-muted mb-2">From phone</div>
          <div className="h-28 bg-[#0f0f14] border border-bridge-border rounded-xl p-3 text-sm text-white overflow-auto">{remote || "No clipboard yet"}</div>
          <div className="text-xs text-bridge-muted mt-2">Images auto-sync via QUIC. History planned.</div>
        </div>
      </div>
    </div>
  )
}
