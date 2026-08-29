package com.bridge.android.telephony

import android.telecom.Connection
import android.telecom.ConnectionRequest
import android.telecom.ConnectionService
import android.telecom.PhoneAccountHandle
import android.telecom.TelecomManager
import android.util.Log

/**
 * Self-managed ConnectionService for Bridge VoIP-like bridging.
 * This allows Bridge to present calls as if they were carrier calls, even if we bridge via WebRTC audio.
 * Requires android.permission.MANAGE_OWN_CALLS + RoleManager DEFAULT_DIALER (or at least self-managed).
 * For carrier calls we use TelecomManager.placeCall, not this service directly.
 * This stub is for future VoIP bridging; it shows we can create a Connection and setActive.
 */
class BridgeConnectionService : ConnectionService() {

    companion object {
        @Volatile var instance: BridgeConnectionService? = null
            private set
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        Log.i("BridgeConnectionService", "created")
    }

    override fun onDestroy() {
        instance = null
        super.onDestroy()
    }

    override fun onCreateOutgoingConnection(
        connectionManagerPhoneAccount: PhoneAccountHandle?,
        request: ConnectionRequest?
    ): Connection? {
        val number = request?.address?.schemeSpecificPart ?: "unknown"
        Log.i("BridgeConnectionService", "createOutgoing $number via $connectionManagerPhoneAccount")
        val conn = BridgeConnection(number)
        conn.setAddress(request?.address, TelecomManager.PRESENTATION_ALLOWED)
        conn.setInitializing()
        // Simulate dialing → active after 500ms
        android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
            try {
                conn.setDialing()
                conn.setActive()
                Log.i("BridgeConnectionService", "outgoing active $number")
            } catch (_: Exception) {}
        }, 500)
        return conn
    }

    override fun onCreateIncomingConnection(
        connectionManagerPhoneAccount: PhoneAccountHandle?,
        request: ConnectionRequest?
    ): Connection? {
        val number = request?.address?.schemeSpecificPart ?: "unknown"
        Log.i("BridgeConnectionService", "createIncoming $number")
        val conn = BridgeConnection(number)
        conn.setAddress(request?.address, TelecomManager.PRESENTATION_ALLOWED)
        conn.setRinging()
        return conn
    }

    class BridgeConnection(private val number: String): Connection() {
        init {
            connectionProperties = PROPERTY_SELF_MANAGED
            audioModeIsVoip = true
        }
        override fun onAnswer() {
            super.onAnswer()
            try {
                setActive()
                Log.i("BridgeConnection", "answered $number -> active")
            } catch (_: Exception) {}
        }
        override fun onDisconnect() {
            super.onDisconnect()
            try {
                setDisconnected(android.telecom.DisconnectCause(android.telecom.DisconnectCause.LOCAL))
                destroy()
                Log.i("BridgeConnection", "disconnected $number")
            } catch (_: Exception) {}
        }
        override fun onReject() {
            super.onReject()
            try {
                setDisconnected(android.telecom.DisconnectCause(android.telecom.DisconnectCause.REJECTED))
                destroy()
            } catch (_: Exception) {}
        }
        override fun onHold() {
            super.onHold()
            try { setOnHold() } catch(_: Exception){}
        }
        override fun onUnhold() {
            super.onUnhold()
            try { setActive() } catch(_: Exception){}
        }
    }
}
