import { useEffect, useRef, useState, useCallback } from 'react'
import { bridge } from '../lib/bridge'
import { canvasToNorm, validateInputEvent, shouldThrottle } from '../lib/control'

export function RemoteControl() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [displayInfo, setDisplayInfo] = useState<{width:number,height:number,displayId:number}|null>(null)
  const [frameB64, setFrameB64] = useState<string>("")
  const [state, setState] = useState<"DISABLED"|"ENABLED"|"CONTROLLING"|"PAUSED">("DISABLED")
  const [throttled, setThrottled] = useState(0)
  const [lastAck, setLastAck] = useState<string>("")
  const lastTsRef = useRef<number|null>(null)
  const isDraggingRef = useRef(false)

  // Display scaling: letterbox
  const display = displayInfo ? {width: displayInfo.width, height: displayInfo.height} : {width:1080,height:2400}

  useEffect(()=>{
    const onDisplayInfo = (m:any)=>{
      const p = m.payload
      if (p.displays) {
        const primary = p.displays.find((d:any)=>d.isPrimary) || p.displays[0]
        if (primary) setDisplayInfo({width:primary.width,height:primary.height,displayId:primary.displayId})
      } else if (p.width) {
        setDisplayInfo({width:p.width,height:p.height,displayId:p.displayId||0})
      }
      setState("ENABLED")
    }
    const onFrame = (m:any)=>{
      const b64 = m.payload.frame_b64
      if (b64) {
        setFrameB64(b64)
        // draw on canvas
        if (canvasRef.current) {
          const canvas = canvasRef.current
          const ctx = canvas.getContext('2d')
          if (ctx) {
            const img = new Image()
            img.onload = ()=>{
              canvas.width = display.width
              canvas.height = display.height
              // letterbox scaling: draw image centered
              ctx.fillStyle = '#0f0f14'
              ctx.fillRect(0,0,canvas.width,canvas.height)
              ctx.drawImage(img,0,0,canvas.width,canvas.height)
            }
            // support both raw base64 and data url
            img.src = b64.startsWith("data:") ? b64 : `data:image/jpeg;base64,${b64}`
          }
        }
      }
    }
    const onAck = (m:any)=>{
      setLastAck(JSON.stringify(m.payload).slice(0,120))
      if (m.payload.throttled) setThrottled(c=>c+1)
    }
    const onControl = (m:any)=>{
      const s = m.payload.state || m.payload.ok ? "CONTROLLING" : "ENABLED"
      if (m.type === "control.start") setState("CONTROLLING")
      if (m.type === "control.stop") setState(m.payload.state || "ENABLED")
      if (m.type === "input.ack" && m.payload.throttled) setThrottled(c=>c+1)
    }
    const onError = (m:any)=>{
      const code = m.payload.code
      if (code === "device_locked") setState("PAUSED")
      if (code === "missing_permission") setState("DISABLED")
      setLastAck(`error ${code}: ${m.payload.message||m.payload.error}`)
    }
    bridge.on("display.info", onDisplayInfo)
    bridge.on("display.frame", onFrame)
    bridge.on("input.ack", onAck)
    bridge.on("control.start", onControl)
    bridge.on("control.stop", onControl)
    bridge.on("error", onError)
    // request display info on mount if connected
    if (bridge.ws && bridge.ws.readyState===1) {
      bridge.send("display.info", {})
    }
    return ()=>{
      bridge.off("display.info", onDisplayInfo)
      bridge.off("display.frame", onFrame)
      bridge.off("input.ack", onAck)
      bridge.off("control.start", onControl)
      bridge.off("control.stop", onControl)
      bridge.off("error", onError)
    }
  },[display.width, display.height])

  const sendInput = useCallback((payload:any)=>{
    const err = validateInputEvent(payload)
    if (err) {
      setLastAck(`validation: ${err}`)
      return
    }
    const now = Date.now()
    // throttle 60fps for move
    if (payload.action === "move" && shouldThrottle(lastTsRef.current, now, 16)) {
      setThrottled(c=>c+1)
      return
    }
    lastTsRef.current = now
    // clamp 0..1
    if (payload.x !== undefined) payload.x = Math.max(0, Math.min(1, payload.x))
    if (payload.y !== undefined) payload.y = Math.max(0, Math.min(1, payload.y))
    bridge.send("input.event", {...payload, ts: now})
  },[])

  const handleCanvasEvent = (e: React.MouseEvent<HTMLCanvasElement>, action: "tap"|"down"|"move"|"up")=>{
    if (!canvasRef.current) return
    const rect = canvasRef.current.getBoundingClientRect()
    const norm = canvasToNorm(e.clientX, e.clientY, rect, display, true)
    sendInput({x:norm.x, y:norm.y, action, displayId: displayInfo?.displayId || 0, pointerId:0, pressure:0.5})
  }

  const startControl = ()=>{
    bridge.send("control.start", {displayId: displayInfo?.displayId || 0, quality:80, fps:30})
    setState("CONTROLLING")
  }
  const stopControl = ()=>{
    bridge.send("control.stop", {displayId: displayInfo?.displayId||0, reason:"user"})
    setState("ENABLED")
  }

  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-white font-semibold">Remote Control — Input Injection</h3>
        <span className={`text-xs px-3 py-1 rounded-full border ${state==="CONTROLLING"?"bg-emerald-500/20 text-emerald-400 border-emerald-500/30":state==="PAUSED"?"bg-amber-500/20 text-amber-400":state==="ENABLED"?"bg-blue-500/20 text-blue-400":"bg-zinc-800 text-zinc-400 border-zinc-700"}`}>{state}</span>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2 bg-[#0f0f14] border border-bridge-border rounded-xl p-3">
          <div className="text-xs text-bridge-muted mb-2">Screen Mirror — canvas {display.width}x{display.height} {displayInfo?`displayId ${displayInfo.displayId}`:"no info"} {throttled?`· throttled ${throttled}`:""}</div>
          <div className="relative aspect-[9/16] max-h-[520px] bg-black rounded-xl overflow-hidden border border-bridge-border">
            <canvas
              ref={canvasRef}
              width={display.width}
              height={display.height}
              className="w-full h-full object-contain bg-black cursor-crosshair"
              onMouseDown={(e)=>{ isDraggingRef.current=true; handleCanvasEvent(e,"down") }}
              onMouseMove={(e)=>{ if(isDraggingRef.current) handleCanvasEvent(e,"move"); }}
              onMouseUp={(e)=>{ isDraggingRef.current=false; handleCanvasEvent(e,"up") }}
              onClick={(e)=>{ if(!isDraggingRef.current) handleCanvasEvent(e,"tap") }}
              onDoubleClick={(e)=>{
                const rect = (e.target as HTMLCanvasElement).getBoundingClientRect()
                const norm = canvasToNorm(e.clientX,e.clientY,rect,display,true)
                sendInput({x:norm.x,y:norm.y,action:"pinch",scale:1.5, displayId: displayInfo?.displayId||0})
              }}
            />
            {!frameB64 && (
              <div className="absolute inset-0 flex items-center justify-center text-xs text-bridge-muted p-4 text-center">
                No frame yet — start control to stream display.frame<br/>Ensure phone toggle "Allow input control" ON and Accessibility enabled
              </div>
            )}
            <div className="absolute bottom-2 right-2 text-[10px] text-white/60 bg-black/60 px-2 py-1 rounded">
              {display.width}x{display.height} · {state}
            </div>
          </div>
          <div className="flex gap-2 mt-3">
            <button onClick={startControl} disabled={state==="CONTROLLING"} className={`flex-1 text-sm px-3 py-2 rounded-xl ${state==="CONTROLLING"?"bg-zinc-800 text-zinc-500":"bg-bridge-accent text-white"}`}>Start Control</button>
            <button onClick={stopControl} disabled={state!=="CONTROLLING"} className={`flex-1 text-sm px-3 py-2 rounded-xl border ${state==="CONTROLLING"?"border-red-500/30 bg-red-500/20 text-red-400":"border-bridge-border text-zinc-500"}`}>Stop</button>
            <button onClick={()=>bridge.send("display.info", {})} className="border border-bridge-border text-white px-3 py-2 rounded-xl text-sm">Refresh Display</button>
          </div>
          <div className="flex gap-2 mt-2">
            <button onClick={()=>sendInput({action:"home"})} className="border border-bridge-border text-white px-3 py-1.5 rounded-xl text-xs">Home</button>
            <button onClick={()=>sendInput({action:"back"})} className="border border-bridge-border text-white px-3 py-1.5 rounded-xl text-xs">Back</button>
            <button onClick={()=>sendInput({action:"key",keyCode:4})} className="border border-bridge-border text-white px-3 py-1.5 rounded-xl text-xs">Key 4 (Back)</button>
            <button onClick={()=>{
              const rect = canvasRef.current?.getBoundingClientRect()
              if(rect) {
                const norm = canvasToNorm(rect.left+rect.width/2, rect.top+rect.height/2, rect, display, true)
                sendInput({x:norm.x,y:norm.y,action:"swipe", durationMs:300, displayId: displayInfo?.displayId||0})
              }
            }} className="border border-bridge-border text-white px-3 py-1.5 rounded-xl text-xs">Swipe Center</button>
          </div>
          <div className="text-xs text-bridge-muted mt-2">Desktop capture via canvas mouse · Throttle 60fps · rdev/enigo stub (Tauri plugin later) · Coalesce moves · Clamp 0..1</div>
        </div>

        <div className="space-y-3">
          <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
            <div className="text-xs text-bridge-muted mb-2">Display Info</div>
            {displayInfo ? (
              <div className="text-xs text-white space-y-1">
                <div>displayId: {displayInfo.displayId}</div>
                <div>{displayInfo.width} x {displayInfo.height} px</div>
                <div>Aspect: {(displayInfo.width/displayInfo.height).toFixed(2)}</div>
                <div>Letterbox canvas maps norm 0..1 → px via DisplayManager metrics</div>
              </div>
            ) : (
              <div className="text-xs text-bridge-muted">No display.info — tap Refresh or Start Control</div>
            )}
            <button onClick={()=>bridge.send("display.info", {displayId:0})} className="mt-3 w-full border border-bridge-border text-white text-xs px-3 py-2 rounded-xl">Get Display Info</button>
            <div className="text-xs text-bridge-muted mt-2">Multi-display: daemon validates displayId via DisplayManager; invalid → error.invalid_display</div>
          </div>

          <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
            <div className="text-xs text-bridge-muted mb-2">Input Ack · Throttle</div>
            <div className="text-xs font-mono text-white break-all min-h-[40px] bg-black rounded p-2">{lastAck || "no ack yet"}</div>
            <div className="text-xs text-bridge-muted mt-2">Throttled dropped: {throttled} · Rate limit 120/s · Audit log ~/.local/share/bridge/audit.log (no coords)</div>
            <div className="flex gap-2 mt-2">
              <button onClick={()=>{ setThrottled(0); setLastAck("")}} className="text-xs text-bridge-muted hover:text-white">Clear</button>
              <button onClick={()=>{
                // spam test
                for(let i=0;i<10;i++) sendInput({x:0.5+Math.random()*0.01,y:0.5,action:"move",displayId:0})
              }} className="text-xs text-bridge-muted hover:text-white">Spam 10 moves (throttle test)</button>
            </div>
          </div>

          <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
            <div className="text-xs text-bridge-muted mb-2">Threat Model</div>
            <div className="text-xs text-bridge-muted space-y-1">
              <div>• Toggle OFF by default</div>
              <div>• Auto-off on lock</div>
              <div>• No background injection</div>
              <div>• BIND_ACCESSIBILITY_SERVICE only via Settings</div>
              <div>• Throttle + clamp + audit</div>
            </div>
            <div className="text-xs text-emerald-400 mt-2">Security: explicit consent, lock check, no silent control</div>
          </div>
        </div>
      </div>
    </div>
  )
}
