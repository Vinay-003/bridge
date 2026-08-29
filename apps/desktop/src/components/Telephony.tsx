import { useEffect, useState, useCallback } from 'react'
import { bridge } from '../lib/bridge'
import { useBridgeStore, type SmsMessage, type CallEntry } from '../lib/store'
import { isValidNumber, isValidSmsBody } from '../lib/telephony'

export function Telephony() {
  const sms = useBridgeStore(s => s.sms)
  const calls = useBridgeStore(s => s.calls)
  const callState = useBridgeStore(s => s.callState)
  const activeCall = useBridgeStore(s => s.activeCall)
  const [number, setNumber] = useState('')
  const [smsTo, setSmsTo] = useState('')
  const [smsBody, setSmsBody] = useState('')
  const [subId, setSubId] = useState<number | undefined>(undefined)
  const [error, setError] = useState<string | null>(null)
  const [dialPadOpen, setDialPadOpen] = useState(true)

  const refreshSms = useCallback(() => {
    bridge.send('sms.list', { limit: 50, offset: 0, subscriptionId: subId ?? null })
  }, [subId])
  const refreshCalls = useCallback(() => {
    bridge.send('call.log', { limit: 50 })
  }, [])

  useEffect(() => {
    const onSmsList = (m: any) => {
      const p = m.payload
      if (Array.isArray(p.messages)) {
        const msgs: SmsMessage[] = p.messages.map((x: any) => ({
          id: String(x.id), address: String(x.address), body: String(x.body), date: Number(x.date), type: Number(x.type ?? 1), read: Number(x.read ?? 0), subscriptionId: x.subscriptionId
        }))
        useBridgeStore.getState().set({ sms: msgs })
      }
      if (Array.isArray(p.subscriptions) && p.subscriptions.length > 0) {
        // auto-select first sub if not set
        if (subId === undefined) setSubId(p.subscriptions[0].id)
      }
    }
    const onSmsSend = (m: any) => {
      // ack from daemon/phone
      if (m.payload?.status === 'relayed') {
        setError(null)
        refreshSms()
      }
      if (m.payload?.code) setError(m.payload.error || m.payload.code)
    }
    const onSmsReceived = (m: any) => {
      const p = m.payload?.payload ?? m.payload
      if (p?.address && p?.body) {
        const cur = useBridgeStore.getState().sms
        const entry: SmsMessage = { id: String(Date.now()), address: String(p.address), body: String(p.body), date: Date.now(), type: 1, read: 0 }
        useBridgeStore.getState().set({ sms: [entry, ...cur].slice(0, 100) })
      }
    }
    const onCallStart = (m: any) => {
      const p = m.payload
      if (p.state === 'RINGING' && p.number) {
        useBridgeStore.getState().set({ callState: 'RINGING', activeCall: { id: String(p.callId), number: String(p.number), state: 'RINGING' } })
      }
      if (p.error || p.code) setError(p.error || p.code)
    }
    const onCallAnswer = (m: any) => {
      useBridgeStore.getState().set({ callState: 'OFFHOOK', activeCall: { ...(useBridgeStore.getState().activeCall ?? { id: m.payload.callId, number: '' }), state: 'OFFHOOK', id: String(m.payload.callId) } as any })
    }
    const onCallHangup = (m: any) => {
      useBridgeStore.getState().set({ callState: 'HUNGUP', activeCall: m.payload.callId ? { id: String(m.payload.callId), number: activeCall?.number ?? '', state: 'HUNGUP' } : null })
      setTimeout(() => useBridgeStore.getState().set({ callState: 'IDLE', activeCall: null }), 1200)
      refreshCalls()
    }
    const onCallLog = (m: any) => {
      if (Array.isArray(m.payload.calls)) {
        const entries: CallEntry[] = m.payload.calls.map((c: any) => ({ number: String(c.number), type: String(c.type), date: Number(c.date), duration: Number(c.duration), subscriptionId: c.subscriptionId }))
        useBridgeStore.getState().set({ calls: entries })
      }
    }
    const onCallAudio = (m: any) => {
      // WebRTC audio stub — in real impl, would setRemoteDescription and bridge to PipeWire
      // console.log('call.audio', m.payload)
    }
    const onError = (m: any) => {
      if (m.payload?.code === 'rate_limited' || m.payload?.code === 'invalid_number' || m.payload?.code === 'missing_permission') {
        setError(m.payload.message || m.payload.error || m.payload.code)
      }
    }
    bridge.on('sms.list', onSmsList)
    bridge.on('sms.send', onSmsSend)
    bridge.on('sms.received', onSmsReceived)
    bridge.on('call.start', onCallStart)
    bridge.on('call.answer', onCallAnswer)
    bridge.on('call.hangup', onCallHangup)
    bridge.on('call.log', onCallLog)
    bridge.on('call.audio', onCallAudio)
    bridge.on('error', onError)
    // initial load
    refreshSms()
    refreshCalls()
    return () => {
      bridge.off('sms.list', onSmsList)
      bridge.off('sms.send', onSmsSend)
      bridge.off('sms.received', onSmsReceived)
      bridge.off('call.start', onCallStart)
      bridge.off('call.answer', onCallAnswer)
      bridge.off('call.hangup', onCallHangup)
      bridge.off('call.log', onCallLog)
      bridge.off('call.audio', onCallAudio)
      bridge.off('error', onError)
    }
  }, [refreshSms, refreshCalls, subId, activeCall])

  const handleDial = () => {
    if (!isValidNumber(number)) { setError('Invalid number: E.164 7-15 digits'); return }
    setError(null)
    bridge.send('call.start', { number, subscriptionId: subId ?? null })
    useBridgeStore.getState().set({ callState: 'RINGING', activeCall: { id: 'pending', number, state: 'RINGING' } })
  }
  const handleHangup = () => {
    const id = activeCall?.id ?? 'pending'
    bridge.send('call.hangup', { callId: id })
    useBridgeStore.getState().set({ callState: 'HUNGUP' })
  }
  const handleAnswer = () => {
    const id = activeCall?.id ?? 'pending'
    bridge.send('call.answer', { callId: id })
    useBridgeStore.getState().set({ callState: 'OFFHOOK' })
  }
  const handleSmsSend = () => {
    if (!isValidNumber(smsTo)) { setError('Invalid SMS address'); return }
    if (!isValidSmsBody(smsBody)) { setError('SMS body 1..918 chars required'); return }
    setError(null)
    bridge.send('sms.send', { address: smsTo, body: smsBody, subscriptionId: subId ?? null })
    setSmsBody('')
  }

  const dialButtons = ['1','2','3','4','5','6','7','8','9','*','0','#']

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="bg-bridge-card border border-bridge-border rounded-2xl p-5">
        <div className="flex items-center justify-between">
          <h3 className="text-white font-semibold">Telephony — Calls & SMS</h3>
          <span className={`text-xs px-3 py-1 rounded-full border ${callState==='IDLE'?'border-zinc-700 text-zinc-400': callState==='RINGING'?'border-amber-500 text-amber-400 animate-pulse': callState==='OFFHOOK'?'border-emerald-500 text-emerald-400':'border-red-500 text-red-400'}`}>{callState} {activeCall ? `· ${activeCall.number}` : ''}</span>
        </div>
        {error && <div className="mt-3 text-xs text-red-400 bg-red-950/30 border border-red-900 rounded-lg p-2">{error}</div>}
        <div className="mt-2 text-xs text-bridge-muted">Per-call explicit tap required on phone · SMS preview requires phone unlocked · Dual-SIM via SubscriptionManager</div>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-12 gap-6">
        {/* Dialer */}
        <div className="xl:col-span-5 bg-bridge-card border border-bridge-border rounded-2xl p-5">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-white font-medium">Dialer</h4>
            <button onClick={()=>setDialPadOpen(v=>!v)} className="text-xs text-bridge-muted border border-bridge-border px-3 py-1 rounded-full">{dialPadOpen?'Hide pad':'Show pad'}</button>
          </div>
          <div className="flex gap-2 mb-3">
            <input value={number} onChange={e=>setNumber(e.target.value)} placeholder="+33612345678" className="flex-1 bg-[#0f0f14] border border-bridge-border rounded-xl px-3 py-2 text-white text-sm placeholder:text-zinc-600" />
            <button onClick={() => setNumber(s=>s.slice(0,-1))} className="text-xs border border-bridge-border text-white px-3 rounded-xl">⌫</button>
          </div>
          <div className="flex gap-2 mb-3">
            <select value={subId ?? ''} onChange={e=>setSubId(e.target.value ? Number(e.target.value) : undefined)} className="bg-[#0f0f14] border border-bridge-border rounded-xl px-2 py-1 text-xs text-white">
              <option value="">Default SIM</option>
              <option value="1">SIM 1</option>
              <option value="2">SIM 2</option>
            </select>
            <button onClick={refreshCalls} className="text-xs text-bridge-muted border border-bridge-border px-3 py-1 rounded-full">Refresh log</button>
          </div>
          {dialPadOpen && (
            <div className="grid grid-cols-3 gap-2 mb-4">
              {dialButtons.map(d=>(
                <button key={d} onClick={()=>setNumber(s=>s+d)} className="h-12 bg-[#0f0f14] border border-bridge-border rounded-xl text-white font-medium hover:bg-bridge-accent/20">{d}</button>
              ))}
            </div>
          )}
          <div className="flex gap-2">
            {callState==='IDLE' || callState==='HUNGUP' ? (
              <button onClick={handleDial} disabled={!isValidNumber(number)} className="flex-1 bg-emerald-600 hover:bg-emerald-700 disabled:bg-zinc-800 disabled:text-zinc-500 text-white py-3 rounded-xl font-medium">📞 Call via Phone</button>
            ) : callState==='RINGING' ? (
              <>
                <button onClick={handleAnswer} className="flex-1 bg-emerald-600 text-white py-3 rounded-xl">Answer</button>
                <button onClick={handleHangup} className="flex-1 bg-red-600 text-white py-3 rounded-xl">Hangup</button>
              </>
            ) : (
              <button onClick={handleHangup} className="flex-1 bg-red-600 text-white py-3 rounded-xl">Hangup ({activeCall?.number})</button>
            )}
          </div>
          <div className="mt-3 text-xs text-bridge-muted">WebRTC audio → PipeWire Bridge Mic/Speaker (Opus 48kHz). Requires phone tap before dial.</div>

          {/* Call log */}
          <div className="mt-6">
            <h5 className="text-sm text-white font-medium mb-2">Call Log</h5>
            {calls.length===0 ? <div className="text-xs text-bridge-muted bg-[#0f0f14] border border-bridge-border rounded-xl p-3">No calls yet — start a call or tap Refresh</div> : (
              <div className="space-y-2 max-h-64 overflow-auto">
                {calls.map((c,i)=>(
                  <div key={i} className="flex justify-between items-center bg-[#0f0f14] border border-bridge-border rounded-xl p-2">
                    <div>
                      <div className="text-sm text-white">{c.number} <span className="text-xs text-bridge-muted">{c.type}</span></div>
                      <div className="text-xs text-bridge-muted">{new Date(c.date).toLocaleString()} · {c.duration}s {c.subscriptionId ? `· SIM${c.subscriptionId}` : ''}</div>
                    </div>
                    <button onClick={()=>setNumber(c.number)} className="text-xs border border-bridge-border px-2 py-1 rounded-full text-white">Redial</button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* SMS */}
        <div className="xl:col-span-7 bg-bridge-card border border-bridge-border rounded-2xl p-5">
          <h4 className="text-white font-medium mb-3">SMS</h4>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-2 mb-3">
            <input value={smsTo} onChange={e=>setSmsTo(e.target.value)} placeholder="To: +336..." className="bg-[#0f0f14] border border-bridge-border rounded-xl px-3 py-2 text-white text-sm placeholder:text-zinc-600" />
            <input value={smsBody} onChange={e=>setSmsBody(e.target.value)} placeholder="Message (1..918 chars)" maxLength={918} className="md:col-span-2 bg-[#0f0f14] border border-bridge-border rounded-xl px-3 py-2 text-white text-sm placeholder:text-zinc-600" />
          </div>
          <div className="flex items-center gap-2 mb-4">
            <span className="text-xs text-bridge-muted">{smsBody.length}/918</span>
            <button onClick={handleSmsSend} disabled={!isValidNumber(smsTo) || !isValidSmsBody(smsBody)} className="ml-auto bg-bridge-accent hover:bg-bridge-accent/90 disabled:bg-zinc-800 disabled:text-zinc-500 text-white px-4 py-2 rounded-xl text-sm">Send via Phone</button>
            <button onClick={refreshSms} className="text-xs border border-bridge-border text-white px-3 py-2 rounded-xl">Refresh</button>
          </div>
          <div className="space-y-2 max-h-[520px] overflow-auto">
            {sms.length===0 ? <div className="text-xs text-bridge-muted bg-[#0f0f14] border border-bridge-border rounded-xl p-6 text-center">No SMS yet — tap Refresh when phone is unlocked (SMS preview requires unlock per threat model).</div> : sms.map(m=>(
              <div key={m.id} className={`border rounded-xl p-3 ${m.type===2?'bg-bridge-accent/10 border-bridge-accent/30 ml-8':'bg-[#0f0f14] border-bridge-border mr-8'}`}>
                <div className="flex justify-between">
                  <span className="text-xs text-bridge-accent2">{m.address} {m.subscriptionId ? `· SIM${m.subscriptionId}` : ''}</span>
                  <span className="text-xs text-bridge-muted">{new Date(m.date).toLocaleString()}</span>
                </div>
                <div className="text-sm text-white mt-1 whitespace-pre-wrap break-words">{m.body}</div>
                <div className="text-xs text-bridge-muted mt-1">{m.type===2 ? 'Sent' : m.type===1 ? 'Inbox' : 'Draft'} · {m.read===1?'Read':'Unread'}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
