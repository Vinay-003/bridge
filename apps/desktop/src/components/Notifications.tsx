import { useEffect } from 'react'
import { bridge } from '../lib/bridge'
import { useBridgeStore } from '../lib/store'

export function Notifications() {
  const notifs = useBridgeStore(s=>s.notifs)
  useEffect(()=>{
    bridge.on("notify.new", (m)=> {
      const n = m.payload;
      useBridgeStore.getState().set({notifs: [{key:n.key||crypto.randomUUID(), app:n.app||"Bridge", title:n.title||"Test", body:n.body||"", ts:Date.now(), hasReply:!!n.hasReply}, ...useBridgeStore.getState().notifs].slice(0,20)})
      if("Notification" in window && Notification.permission==="granted") new Notification(n.title||"Bridge",{body:n.body})
    })
    // demo
    if(notifs.length===0) {
      useBridgeStore.getState().set({notifs:[
        {key:"1", app:"WhatsApp", title:"Mom", body:"Call me when free", ts:Date.now(), hasReply:true},
        {key:"2", app:"Gmail", title:"OTP", body:"Your code is 493211", ts:Date.now(), hasReply:false},
      ]})
    }
  },[])
  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-white font-semibold">Notifications</h3>
        <button onClick={()=>bridge.send("notify.action",{key:"1", action:"dismiss"})} className="text-xs text-bridge-muted">Clear all</button>
      </div>
      <div className="space-y-2 max-h-64 overflow-auto">
        {notifs.map(n=>(
          <div key={n.key} className="bg-[#0f0f14] border border-bridge-border rounded-xl p-3">
            <div className="flex justify-between gap-3">
              <div>
                <div className="text-xs text-bridge-accent2">{n.app}</div>
                <div className="text-sm text-white font-medium">{n.title}</div>
                <div className="text-xs text-bridge-muted">{n.body}</div>
              </div>
              <div className="flex flex-col gap-1">
                {n.hasReply && <button onClick={()=>{
                  const t=prompt("Reply:");
                  if(t) bridge.send("notify.action",{key:n.key, action:"reply", text:t})
                }} className="text-xs bg-bridge-accent text-white px-3 py-1.5 rounded-lg">Reply</button>}
                <button onClick={()=>bridge.send("notify.action",{key:n.key, action:"dismiss"})} className="text-xs border border-bridge-border text-white px-3 py-1.5 rounded-lg">Dismiss</button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
