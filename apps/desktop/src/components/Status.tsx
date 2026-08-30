import { useEffect } from 'react'
import { bridge } from '../lib/bridge'
import { useBridgeStore } from '../lib/store'

export function Status() {
  const s = useBridgeStore(s=>s.status)
  const connected = useBridgeStore(s=>s.connected)
  const phoneConnected = useBridgeStore(s=>s.phoneConnected)
  const daemonConnected = useBridgeStore(s=>s.connected)
  useEffect(()=>{
    let phoneTimer: ReturnType<typeof setTimeout> | null = null
    const onStatus = (m:any)=> {
      const src = m.payload.source
      useBridgeStore.getState().set({status: m.payload})
      if (src === "phone") {
        useBridgeStore.getState().set({phoneConnected: true})
        if (phoneTimer) clearTimeout(phoneTimer)
        phoneTimer = setTimeout(()=> useBridgeStore.getState().set({phoneConnected: false}), 12000)
      } else if (src === "daemon" && !useBridgeStore.getState().phoneConnected) {
        // keep phoneConnected false if only daemon status
      }
    }
    bridge.on("status.push", onStatus)
    // Also handle explicit disconnect via WS close
    const onWsState = (s:string)=> {
      if (s === "disconnected") {
        // don't immediately flip phone, wait for timeout
      }
    }
    // poll for phone disconnect if no status for 12s

    // if no push after 4s, show local fallback as disconnected hint
    const t = setTimeout(()=>{
      if(!useBridgeStore.getState().status) {
        useBridgeStore.getState().set({status: {
          battery:{pct:0,charging:false,tempC:0}, ram:{availMb:0,totalMb:0}, storage:{freeGb:0,totalGb:0}, signal:{dbm:0,bars:0}
        }})
      }
    }, 4000)
    return ()=> { bridge.off("status.push", onStatus); clearTimeout(t); if (phoneTimer) clearTimeout(phoneTimer); }
  },[])
  if(!s || s.battery.pct===0) {
    return (
      <div className="grid grid-cols-4 gap-3">
        <div className="bg-bridge-card border border-bridge-border rounded-2xl p-4 text-center col-span-4">
          <div className="text-xs text-bridge-muted">{connected ? "Waiting for phone status (phone must be connected — check Pair tab)" : "Daemon not connected — start daemon on 8443"}</div>
        </div>
      </div>
    )
  }
  return (
    <div className="grid grid-cols-4 gap-3">
      {[
        {k:"Battery", v:`${s.battery.pct}%`, sub:s.battery.charging?"Charging":"Discharging"},
        {k:"Temp", v:`${s.battery.tempC}°C`, sub:`${s.signal.bars} bars`},
        {k:"RAM", v:`${s.ram.availMb} MB`, sub:`/${s.ram.totalMb}`},
        {k:"Storage", v:`${s.storage.freeGb} GB`, sub:`free / ${s.storage.totalGb}`},
      ].map(c=>(
        <div key={c.k} className="bg-bridge-card border border-bridge-border rounded-2xl p-4">
          <div className="text-xs text-bridge-muted">{c.k}</div>
          <div className="text-lg font-semibold text-white">{c.v}</div>
          <div className="text-xs text-bridge-muted">{c.sub}</div>
        </div>
      ))}
    </div>
  )
}
