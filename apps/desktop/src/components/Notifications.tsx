import { useEffect } from 'react'
import { bridge } from '../lib/bridge'
import { useBridgeStore } from '../lib/store'

export function Notifications() {
  const notifs = useBridgeStore(s=>s.notifs)
  useEffect(()=>{
    const onNew = (m:any)=> {
      const n = m.payload;
      const entry = {
        key: n.key || crypto.randomUUID(),
        app: n.app || "Bridge",
        title: n.title || "No title",
        body: n.body || n.text || "",
        ts: n.ts || Date.now(),
        hasReply: !!n.hasReply || !!n.replyAction
      }
      const cur = useBridgeStore.getState().notifs
      if(cur.find(x=>x.key===entry.key)) return
      useBridgeStore.getState().set({notifs: [entry, ...cur].slice(0,20)})
      if("Notification" in window && Notification.permission==="granted") {
        try { new Notification(entry.title,{body: entry.body}) } catch {}
      }
    }
    bridge.on("notify.new", onNew)
    bridge.on("notify.new", onNew) // also handle file? idempotent
    // demo fallback only if no real notifs after 5s and disconnected
    const t = setTimeout(()=>{
      if(useBridgeStore.getState().notifs.length===0 && !useBridgeStore.getState().connected) {
        // don't show fake when connected but empty (means no phone notifs yet)
      }
    },5000)
    return ()=> { bridge.off("notify.new", onNew); clearTimeout(t); }
  },[])
  // also listen for clipboard-style notify alias
  useEffect(()=>{
    const h = (m:any)=> {
      if(m.type==="notify.new" || m.type==="notify.action") {
        // handled
      }
    }
    bridge.on("*", h)
    return ()=> bridge.off("*", h)
  },[])

  if(notifs.length===0) {
    return (
      <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5">
        <h3 className="text-white font-semibold mb-3">Notifications</h3>
        <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-6 text-center">
          <div className="text-sm text-white">No notifications yet</div>
          <div className="text-xs text-bridge-muted mt-1">Phone notifications appear here after you enable Notification Access on the phone and they auto-reply/dismiss.</div>
          <div className="text-xs text-bridge-muted mt-2">Test: send yourself a WhatsApp/Gmail</div>
        </div>
      </div>
    )
  }

  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-white font-semibold">Notifications</h3>
        <button onClick={()=>useBridgeStore.getState().set({notifs: []})} className="text-xs text-bridge-muted">Clear all</button>
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
                  const t=prompt("Reply to "+n.title+":");
                  if(t) bridge.send("notify.action",{key:n.key, action:"reply", text:t})
                }} className="text-xs bg-bridge-accent text-white px-3 py-1.5 rounded-lg">Reply</button>}
                <button onClick={()=>{
                  bridge.send("notify.action",{key:n.key, action:"dismiss"})
                  useBridgeStore.getState().set({notifs: useBridgeStore.getState().notifs.filter(x=>x.key!==n.key)})
                }} className="text-xs border border-bridge-border text-white px-3 py-1.5 rounded-lg">Dismiss</button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
