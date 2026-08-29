import { useEffect, useState } from 'react'
import { QRCodeSVG } from 'qrcode.react'
import { bridge } from '../lib/bridge'
import { useBridgeStore } from '../lib/store'

export function Pairing() {
  const connected = useBridgeStore(s=>s.connected)
  const [qr, setQr] = useState("")
  const [fp, setFp] = useState("")
  const [sas, setSas] = useState("")
  const [host, setHost] = useState(location.hostname)
  const [showConfirm, setShowConfirm] = useState(false)

  useEffect(()=>{
    const onTrusted = (m:any)=>{
      const p = m.payload
      if(p.qr) setQr(p.qr)
      if(p.fp) setFp(p.fp)
      if(p.sas) setSas(p.sas)
      if(p.host) setHost(p.host)
      setShowConfirm(true)
    }
    const onSas = (m:any)=> { setSas(m.payload.sas || ""); setShowConfirm(true); }
    bridge.on("pairing.trusted", onTrusted)
    bridge.on("pairing.sas", onSas)
    fetch(`http://${location.hostname}:8443/qr`).then(r=>r.json()).then(j=>{
      if(j.qr) setQr(j.qr); if(j.fp) setFp(j.fp); if(j.sas) setSas(j.sas); if(j.host) setHost(j.host);
    }).catch(()=>{})
    const tryHello = ()=> bridge.send("pairing.hello", { client: "desktop" })
    setTimeout(tryHello, 500)
    setTimeout(tryHello, 2000)
    return ()=> { bridge.off("pairing.trusted", onTrusted); bridge.off("pairing.sas", onSas); }
  },[])

  const displayFp = fp ? fp.match(/.{1,2}/g)?.join(":") || fp : "—"
  const displaySas = sas || "—"

  return (
    <div className="rounded-2xl bg-bridge-card border border-bridge-border p-5">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-white font-semibold">Pair Android</h3>
        <span className={`text-xs px-2 py-1 rounded-full ${connected?'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30':'bg-zinc-800 text-zinc-400'}`}>{connected?'Connected':'Not connected'}</span>
      </div>
      <div className="flex flex-col sm:flex-row gap-5">
        <div className="bg-white p-3 rounded-xl w-fit h-fit flex items-center justify-center shrink-0 mx-auto sm:mx-0">
          {qr ? <QRCodeSVG value={qr} size={140} /> : <span className="text-xs text-zinc-500">Waiting daemon…</span>}
        </div>
        <div className="flex-1 space-y-3 min-w-0">
          <p className="text-sm text-bridge-muted">On Android open Bridge → <b className="text-white">Scan QR</b>. Host <code className="text-white">{host}:8443</code>. Code refreshes on daemon restart.</p>
          <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-3 space-y-1">
            <div className="text-xs text-bridge-muted">Fingerprint</div>
            <div className="font-mono text-xs text-white break-all">{displayFp}</div>
            <div className="text-xs text-bridge-muted mt-2">SAS</div>
            <div className="font-mono text-xl tracking-[0.3em] text-bridge-accent2">{displaySas}</div>
            <div className="text-xs text-bridge-muted mt-1 truncate" title={qr}>QR {qr.slice(0,60)}…</div>
          </div>
          <div className="flex gap-2 flex-wrap">
            {showConfirm && <button onClick={()=>bridge.send("pairing.sas",{confirm:true, sas})} className="bg-bridge-accent text-white px-4 py-2 rounded-xl text-sm">Confirm SAS matches</button>}
            <button onClick={()=>{
              if(bridge.ws) bridge.ws.close()
              setTimeout(()=>bridge.connect(), 500)
            }} className="border border-bridge-border text-white px-4 py-2 rounded-xl text-sm">Reconnect</button>
            <button onClick={()=>{
              if(bridge.ws) bridge.ws.close()
              useBridgeStore.getState().set({connected:false})
            }} className="border border-red-500/30 text-red-400 px-4 py-2 rounded-xl text-sm">Stop / Disconnect</button>
          </div>
          <div className="text-xs text-bridge-muted">USB fallback: <code className="text-white">adb forward tcp:8443 tcp:8443</code></div>
        </div>
      </div>
    </div>
  )
}
