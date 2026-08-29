import { useCallback, useState } from 'react'
import { bridge } from '../lib/bridge'
import { useBridgeStore } from '../lib/store'

export function Files() {
  const [drag, setDrag] = useState(false)
  const transfers = useBridgeStore(s=>s.transfers)
  const sendFile = useCallback(async (file: File)=>{
    const id = crypto.randomUUID()
    const chunkSize = 1024*1024
    const total = Math.ceil(file.size / chunkSize)
    let offset=0, idx=0
    for (; offset < file.size; offset+=chunkSize, idx++) {
      const slice = file.slice(offset, offset+chunkSize)
      const buf = await slice.arrayBuffer()
      const b64 = btoa(String.fromCharCode(...new Uint8Array(buf)))
      // compute sha256 for chunk (browser subtle crypto if available)
      let sha = "demo"
      try {
        const hash = await crypto.subtle.digest("SHA-256", buf)
        sha = Array.from(new Uint8Array(hash)).map(b=>b.toString(16).padStart(2,"0")).join("")
      } catch {}
      bridge.send("file.chunk", { id, name:file.name, size:file.size, offset, total, index: idx, sha256: sha, data_b64: b64 })
      const pct = Math.round(((idx+1)/total)*100)
      const existing = useBridgeStore.getState().transfers
      const without = existing.filter(t=>t.id!==id)
      useBridgeStore.getState().set({transfers:[...without, {id, name:file.name, pct, done: pct===100}]})
    }
  },[])
  const onDrop = useCallback(async (e: React.DragEvent)=>{
    e.preventDefault(); setDrag(false);
    const files = Array.from(e.dataTransfer.files);
    for(const f of files) await sendFile(f)
  },[sendFile])
  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5">
      <h3 className="text-white font-semibold mb-3">Files</h3>
      <div onDragOver={e=>{e.preventDefault(); setDrag(true)}} onDragLeave={()=>setDrag(false)} onDrop={onDrop}
        className={`border-2 border-dashed rounded-2xl p-8 text-center ${drag?'border-bridge-accent bg-bridge-accent/10':'border-bridge-border'}`}>
        <p className="text-white">Drag & drop files or folders</p>
        <p className="text-xs text-bridge-muted mt-1">1 MB chunks, SHA256, resume on reconnect. Saved to ~/Bridge</p>
        <input type="file" multiple onChange={async e=>{
          const files = Array.from(e.target.files||[]);
          for(const f of files) await sendFile(f)
        }} className="mt-4 text-xs text-bridge-muted" />
      </div>
      {transfers.length>0 && <div className="mt-4 space-y-2">
        {transfers.slice(-5).reverse().map(t=>(
          <div key={t.id} className="flex items-center justify-between bg-[#0f0f14] border border-bridge-border rounded-xl p-3">
            <span className="text-sm text-white truncate">{t.name}</span>
            <span className={`text-xs ${t.done?'text-emerald-400':'text-bridge-accent'}`}>{t.pct}% {t.done?'✓':''}</span>
          </div>
        ))}
      </div>}
      <div className="text-xs text-bridge-muted mt-3">Files appear in <code className="text-white">~/Bridge/</code> · also test: <code className="text-white">ls ~/Bridge</code></div>
    </div>
  )
}
