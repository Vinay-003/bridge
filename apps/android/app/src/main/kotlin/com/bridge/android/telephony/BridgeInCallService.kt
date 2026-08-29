package com.bridge.android.telephony

import android.telecom.Call
import android.telecom.InCallService
import android.util.Log
import org.json.JSONObject

class BridgeInCallService : InCallService() {

    companion object {
        @Volatile var instance: BridgeInCallService? = null
            private set
    }

    private val calls = mutableMapOf<String, Call>()

    override fun onCreate() {
        super.onCreate()
        instance = this
        Log.i("BridgeInCallService", "created")
    }

    override fun onDestroy() {
        instance = null
        calls.clear()
        super.onDestroy()
    }

    override fun onCallAdded(call: Call?) {
        super.onCallAdded(call)
        call?.let {
            val id = it.details?.handle?.schemeSpecificPart ?: "call-${System.currentTimeMillis()}"
            calls[id] = it
            Log.i("BridgeInCallService", "call added $id state=${it.state}")
            it.registerCallback(object: Call.Callback() {
                override fun onStateChanged(c: Call?, state: Int) {
                    Log.i("BridgeInCallService", "state $id -> $state")
                    // Map to our CallState and broadcast via BridgeService if needed
                    // state: Call.STATE_RINGING=2, ACTIVE=4, DISCONNECTED=7 etc.
                }
            })
        }
    }

    override fun onCallRemoved(call: Call?) {
        super.onCallRemoved(call)
        call?.let {
            val id = it.details?.handle?.schemeSpecificPart ?: ""
            calls.remove(id)
            Log.i("BridgeInCallService", "call removed $id")
        }
    }

    fun answerCallById(callId: String) {
        // Try exact match, else first ringing
        var target = calls[callId]
        if (target == null) {
            target = calls.values.firstOrNull { it.state == Call.STATE_RINGING }
        }
        try {
            target?.answer(android.telecom.VideoProfile.STATE_AUDIO_ONLY)
            Log.i("BridgeInCallService", "answer $callId -> ${target != null}")
        } catch (e: Exception) {
            Log.w("BridgeInCallService", "answer failed $callId: ${e.message}")
        }
    }

    fun hangupCallById(callId: String) {
        var target = calls[callId]
        if (target == null) {
            target = calls.values.firstOrNull { it.state != Call.STATE_DISCONNECTED }
        }
        try {
            target?.disconnect()
            Log.i("BridgeInCallService", "hangup $callId -> ${target != null}")
        } catch (e: Exception) {
            Log.w("BridgeInCallService", "hangup failed $callId: ${e.message}")
        }
    }

    fun handleAudioPayload(payload: JSONObject) {
        // Real impl would decode Opus sdp/ice and route to Connection's audio
        // For now just log; WebRTC would be handled via separate WebRTC stack and Telecom Connection
        Log.i("BridgeInCallService", "handleAudio ${payload.optString("callId")} sdp=${payload.optString("sdp").take(40)}")
        // Could forward to PipeWire via daemon: opus decode → AudioTrack to earpiece
    }
}
