import { useEffect } from 'react'
import { bridge } from '../lib/bridge'
import { useBridgeStore } from '../lib/store'

export function Status() {
  const s = useBridgeStore(s=>s.status)
  useEffect(()=>{
    bridge.on("status.push", (m)=> useBridgeStore.getState().set({status: m.payload}))
    // demo fallback if daemon not running
    if(!s) useBridgeStore.getState().set({status: {
      battery:{pct:87,charging:true,tempC:31}, ram:{availMb:4200,totalMb:15600}, storage:{freeGb:120,totalGb:512}, signal:{dbm:-67,bars:4}
    }})
  },[])
  if(!s) return null
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
