import { useEffect, useRef, useState } from 'react'
import { bridge } from '../lib/bridge'

export function Media() {
  const videoRef = useRef<HTMLVideoElement>(null)
  const [camOn, setCamOn] = useState(false)
  const [micOn, setMicOn] = useState(false)
  const [mirrorUrl, setMirrorUrl] = useState("")

  useEffect(()=>{
    const onAnswer = (m:any)=>{
      console.log("webrtc answer", m.payload)
      if(m.payload.sdp) {
        // For MVP we just show that signalling works; real WebRTC would setRemoteDescription here
      }
    }
    const onIce = (m:any)=> console.log("ice", m.payload)
    bridge.on("webrtc.answer", onAnswer)
    bridge.on("webrtc.ice", onIce)
    return ()=> { bridge.off("webrtc.answer", onAnswer); bridge.off("webrtc.ice", onIce); }
  },[])

  const startMirrorDesktop = async ()=>{
    try {
      const stream = await navigator.mediaDevices.getDisplayMedia({video:true, audio:true})
      if(videoRef.current) videoRef.current.srcObject = stream
      setMirrorUrl("desktop capture active")
    } catch(e){ alert("Screen share denied: "+e) }
  }

  return (
    <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5 space-y-4">
      <h3 className="text-white font-semibold">Media — Camera · Mic · Screen</h3>

      <div className="grid grid-cols-3 gap-4">
        <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
          <div className="text-xs text-bridge-muted mb-2">Phone as Webcam</div>
          <div className="aspect-video bg-black rounded-xl flex items-center justify-center text-xs text-bridge-muted p-3 text-center">
            {camOn? "Signalling WebRTC offer sent → phone should start CameraX H.264. Check ~/Bridge for test capture or v4l2loopback /dev/video10 if phone replies." : "Select 'Bridge Cam' in Meet/Zoom after phone streams"}
          </div>
          <div className="flex gap-2 mt-3">
            <button onClick={()=>{
              const next = !camOn
              setCamOn(next)
              bridge.send("webrtc.offer",{type: next?"webcam_start":"webcam_stop", cam: "front", fps:30, res:"720p", v4l2:"/dev/video10"})
            }} className={`flex-1 text-sm px-3 py-2 rounded-xl ${camOn?'bg-red-500 text-white':'bg-bridge-accent text-white'}`}>{camOn?'Stop':'Start Front'}</button>
            <button onClick={()=>bridge.send("webrtc.offer",{type:"webcam_start", cam:"rear"})} className="border border-bridge-border text-white px-3 py-2 rounded-xl text-sm">Rear</button>
          </div>
          <div className="text-xs text-bridge-muted mt-2">v4l2loopback: <code className="text-white">/dev/video10</code> · Latency target &lt;100ms · daemon creates virtual cam on first offer</div>
        </div>

        <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
          <div className="text-xs text-bridge-muted mb-2">Phone Mic · Speaker</div>
          <div className="h-24 bg-black rounded-xl flex items-center justify-center text-xs text-bridge-muted p-3 text-center">
            {micOn? "Opus 48k → PipeWire Bridge Mic (signalling)" : "Muted — tap Unmute to send mic_start offer"}
          </div>
          <div className="flex gap-2 mt-3">
            <button onClick={()=>{
              const next = !micOn
              setMicOn(next)
              bridge.send("webrtc.offer",{type: next?"mic_start":"mic_stop", ns:true, ec:true, pipewire:"Bridge Mic"})
            }} className={`flex-1 text-sm px-3 py-2 rounded-xl ${micOn?'bg-emerald-600 text-white':'bg-bridge-accent text-white'}`}>{micOn?'Mute':'Unmute Mic'}</button>
            <button onClick={()=>bridge.send("webrtc.offer",{type:"speaker", route:"phone"}) } className="border border-bridge-border text-white px-3 py-2 rounded-xl text-sm">Speaker</button>
          </div>
          <div className="text-xs text-bridge-muted mt-2">Virtual: Bridge Mic / Bridge Speaker · AGC/NS · test with <code className="text-white">pactl list sources</code></div>
        </div>

        <div className="bg-[#0f0f14] border border-bridge-border rounded-xl p-4">
          <div className="text-xs text-bridge-muted mb-2">Screen Mirror</div>
          <video ref={videoRef} autoPlay muted playsInline className="aspect-video bg-black rounded-xl w-full object-contain" />
          {mirrorUrl && <div className="text-xs text-emerald-400 mt-1">{mirrorUrl}</div>}
          <div className="flex gap-2 mt-3">
            <button onClick={startMirrorDesktop} className="flex-1 bg-bridge-accent text-white text-sm px-3 py-2 rounded-xl">Capture Desktop</button>
            <button onClick={()=>bridge.send("webrtc.offer",{type:"mirror", src:"phone"})} className="border border-bridge-border text-white px-3 py-2 rounded-xl text-sm">Phone Mirror</button>
          </div>
          <div className="flex gap-2 mt-2">
            <button onClick={()=>bridge.send("webrtc.offer",{type:"screenshot"})} className="text-xs text-bridge-muted hover:text-white">Screenshot</button>
            <button onClick={()=>bridge.send("webrtc.offer",{type:"record", on:true})} className="text-xs text-bridge-muted hover:text-white">● Record</button>
            <span className="text-xs text-bridge-muted">→ ~/Bridge or /tmp</span>
          </div>
        </div>
      </div>
      <div className="text-xs text-bridge-muted">Real media needs phone to answer webrtc.offer via CameraX/MediaProjection; stub echoes answer now. Full WebRTC SRTP next.</div>
    </div>
  )
}
