import { useEffect, useRef, useState } from 'react'
import { bridge } from '../lib/bridge'

export function Media() {
  const videoRef = useRef<HTMLVideoElement>(null)
  const [camOn, setCamOn] = useState(false)
  const [micOn, setMicOn] = useState(false)

  useEffect(()=>{
    bridge.on("webrtc.answer", (m)=>{
      // mocked answer
      console.log("webrtc answer", m.payload)
    })
  },[])

  const startMirror = async ()=>{
    try {
      const stream = await navigator.mediaDevices.getDisplayMedia({video:true, audio:true})
      if(videoRef.current) videoRef.current.srcObject = stream
    } catch(e){ alert("Screen share denied") }
  }

  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5 space-y-4">
      <h3 className="text-white font-semibold">Media — Camera · Mic · Screen</h3>

      <div className="grid grid-cols-3 gap-4">
        <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
          <div className="text-xs text-bridge-muted mb-2">Phone as Webcam</div>
          <div className="aspect-video bg-black rounded-xl flex items-center justify-center text-xs text-bridge-muted">
            {camOn? "WebRTC H.264 → /dev/video10 (Bridge Cam)" : "Select 'Bridge Cam' in Meet/Zoom"}
          </div>
          <div className="flex gap-2 mt-3">
            <button onClick={()=>{
              setCamOn(!camOn)
              bridge.send(camOn?"webrtc.ice":"webrtc.offer",{type: camOn?"stop":"webcam", cam: camOn?"rear":"front", fps:30, res:"720p"})
            }} className={`flex-1 text-sm px-3 py-2 rounded-xl ${camOn?'bg-red-500 text-white':'bg-bridge-accent text-white'}`}>{camOn?'Stop':'Start Front'}</button>
            <button onClick={()=>bridge.send("webrtc.offer",{type:"webcam", cam:"rear"})} className="border border-bridge-border text-white px-3 py-2 rounded-xl text-sm">Rear</button>
          </div>
          <div className="text-xs text-bridge-muted mt-2">v4l2loopback: /dev/video10 · Latency target &lt;100ms</div>
        </div>

        <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
          <div className="text-xs text-bridge-muted mb-2">Phone Mic · Speaker</div>
          <div className="h-24 bg-black rounded-xl flex items-center justify-center text-xs text-bridge-muted">
            {micOn? "Opus 48k → PipeWire Bridge Mic" : "Muted"}
          </div>
          <div className="flex gap-2 mt-3">
            <button onClick={()=>{
              setMicOn(!micOn)
              bridge.send("webrtc.offer",{type: micOn?"mic-stop":"mic-start", ns:true, ec:true})
            }} className={`flex-1 text-sm px-3 py-2 rounded-xl ${micOn?'bg-emerald-600 text-white':'bg-bridge-accent text-white'}`}>{micOn?'Mute':'Unmute Mic'}</button>
            <button onClick={()=>bridge.send("webrtc.offer",{type:"speaker", route:"phone"}) } className="border border-bridge-border text-white px-3 py-2 rounded-xl text-sm">Speaker</button>
          </div>
          <div className="text-xs text-bridge-muted mt-2">Virtual: Bridge Mic / Bridge Speaker · AGC/NS</div>
        </div>

        <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
          <div className="text-xs text-bridge-muted mb-2">Screen Mirror</div>
          <video ref={videoRef} autoPlay muted playsInline className="aspect-video bg-black rounded-xl w-full object-contain" />
          <div className="flex gap-2 mt-3">
            <button onClick={startMirror} className="flex-1 bg-bridge-accent text-white text-sm px-3 py-2 rounded-xl">Capture Desktop</button>
            <button onClick={()=>bridge.send("webrtc.offer",{type:"mirror", src:"phone"})} className="border border-bridge-border text-white px-3 py-2 rounded-xl text-sm">Phone Mirror</button>
          </div>
          <div className="flex gap-2 mt-2">
            <button onClick={()=>bridge.send("webrtc.offer",{type:"screenshot"})} className="text-xs text-bridge-muted">Screenshot</button>
            <button onClick={()=>bridge.send("webrtc.offer",{type:"record", on:true})} className="text-xs text-bridge-muted">● Record</button>
          </div>
        </div>
      </div>
      <div className="text-xs text-bridge-muted">Remote control (future) requires Android AccessibilityService. All media E2E via TLS + SRTP.</div>
    </div>
  )
}
