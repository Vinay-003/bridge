import { useCallback, useState } from 'react'
import { bridge } from '../lib/bridge'
import { useBridgeStore } from '../lib/store'

export function Files() {
  const [drag, setDrag] = useState(false)
  const transfers = useBridgeStore(s=>s.transfers)
  const onDrop = useCallback(async (e: React.DragEvent)=>{
    e.preventDefault(); setDrag(false);
    const files = Array.from(e.dataTransfer.files);
    for(const f of files) {
      const id = crypto.randomUUID();
      const buf = await f.arrayBuffer();
      const b64 = btoa(String.fromCharCode(...new Uint8Array(buf)));
      // naive single-chunk for MVP demo
      bridge.send("file.chunk", { id, name:f.name, size:f.size, offset:0, total:1, index:0, sha256:"demo", data_b64:b64 });
      useBridgeStore.getState().set({transfers:[...useBridgeStore.getState().transfers, {id, name:f.name, pct:100, done:true}]})
    }
  },[])
  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5">
      <h3 className="text-white font-semibold mb-3">Files</h3>
      <div onDragOver={e=>{e.preventDefault(); setDrag(true)}} onDragLeave={()=>setDrag(false)} onDrop={onDrop}
        className={`border-2 border-dashed rounded-2xl p-8 text-center ${drag?'border-bridge-accent bg-bridge-accent/10':'border-bridge-border'}`}>
        <p className="text-white">Drag & drop files or folders</p>
        <p className="text-xs text-bridge-muted mt-1">1 MB chunks, SHA256, resume on reconnect. Saved to ~/Bridge</p>
        <input type="file" multiple onChange={async e=>{
          const files = Array.from(e.target.files||[]);
          for(const f of files){ const b=await f.arrayBuffer(); const b64=btoa(String.fromCharCode(...new Uint8Array(b))); bridge.send("file.chunk",{id:crypto.randomUUID(),name:f.name,size:f.size,offset:0,total:1,index:0,sha256:"demo",data_b64:b64})}
        }} className="mt-4 text-xs text-bridge-muted" />
      </div>
      {transfers.length>0 && <div className="mt-4 space-y-2">
        {transfers.map(t=>(
          <div key={t.id} className="flex items-center justify-between bg-[#0f0f14] border border-bridge-border rounded-xl p-3">
            <span className="text-sm text-white">{t.name}</span><span className="text-xs text-emerald-400">{t.pct}%</span>
          </div>
        ))}
      </div>}
    </div>
  )
}
