import { useEffect, useState } from 'react'
import { QRCodeSVG } from 'qrcode.react'
import { bridge } from '../lib/bridge'
import { useBridgeStore } from '../lib/store'

export function Pairing() {
  const { connected, pairing } = useBridgeStore()
  const [qr, setQr] = useState("bridge://pair?v=1&id=demo&ecdh=demo&fp=ab:cd:ef&port=8443")
  const [fp, setFp] = useState("ab:cd:ef:12:34")
  const [sas, setSas] = useState("123456")
  const [show, setShow] = useState(false)

  useEffect(()=>{
    // mock pairing generation; real would be invoke('pairing_start')
    bridge.on("pairing.sas", (m)=> { setSas(m.payload.sas || "123456"); setShow(true); })
    bridge.on("pairing.trusted", ()=> setShow(false))
  },[])
  useEffect(()=>{
    // simulate fetch from daemon http fallback: try http://localhost:8443/qr
    fetch(`http://${location.hostname}:8443/qr`).then(r=>r.json()).then(j=>{ setQr(j.qr||qr); setFp(j.fp||fp); setSas(j.sas||sas)}).catch(()=>{})
  },[])

  return (
    <div className="rounded-2xl bg-bridge-card border border-bridge-border p-5">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-white font-semibold">Pair Android</h3>
        <span className={`text-xs px-2 py-1 rounded-full ${connected?'bg-emerald-500/20 text-emerald-400':'bg-zinc-800 text-zinc-400'}`}>{connected?'Connected':'Not connected'}</span>
      </div>
      <div className="flex gap-5">
        <div className="bg-white p-3 rounded-xl">
          <QRCodeSVG value={qr} size={140} />
        </div>
        <div className="flex-1 space-y-3">
          <p className="text-sm text-bridge-muted">On Android open Bridge → <b className="text-white">Scan QR</b>. Ensure same Wi-Fi. Code refreshes every 60s.</p>
          <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-3 space-y-1">
            <div className="text-xs text-bridge-muted">Fingerprint</div>
            <div className="font-mono text-xs text-white">{fp}</div>
            <div className="text-xs text-bridge-muted mt-2">SAS</div>
            <div className="font-mono text-xl tracking-[0.3em] text-bridge-accent2">{sas}</div>
          </div>
          {show && <div className="flex gap-2"><button onClick={()=>bridge.send("pairing.sas",{confirm:true})} className="bg-bridge-accent text-white px-4 py-2 rounded-xl text-sm">Confirm SAS matches</button><button className="border border-bridge-border text-white px-4 py-2 rounded-xl text-sm">Reject</button></div>}
          <div className="text-xs text-bridge-muted">USB fallback: <code className="text-white">adb forward tcp:8443 tcp:8443</code></div>
        </div>
      </div>
    </div>
  )
}
