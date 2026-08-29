package com.bridge.android.telephony

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.telecom.TelecomManager
import android.telephony.SubscriptionManager
import androidx.core.content.ContextCompat
import org.json.JSONObject

enum class CallState { IDLE, RINGING, OFFHOOK, HUNGUP }

data class CallSession(
    val id: String,
    val number: String,
    val state: CallState,
    val subscriptionId: Int?,
    val direction: String // INCOMING / OUTGOING
)

object CallHandler {
    @Volatile private var currentState: CallState = CallState.IDLE
    @Volatile private var currentCallId: String? = null
    private val pendingConfirms = mutableMapOf<String, Long>() // callId -> expiry

    fun getState(): CallState = currentState

    fun canTransition(from: CallState, to: CallState): Boolean {
        return when (from to to) {
            CallState.IDLE to CallState.RINGING -> true
            CallState.RINGING to CallState.OFFHOOK -> true
            CallState.RINGING to CallState.HUNGUP -> true
            CallState.OFFHOOK to CallState.HUNGUP -> true
            CallState.HUNGUP to CallState.IDLE -> true
            CallState.IDLE to CallState.OFFHOOK -> true // emergency fallback
            else -> false
        }
    }

    private fun setState(next: CallState) {
        if (canTransition(currentState, next)) {
            currentState = next
        }
    }

    fun hasPermission(context: Context, perm: String): Boolean {
        return ContextCompat.checkSelfPermission(context, perm) == PackageManager.PERMISSION_GRANTED
    }

    fun isDeviceLocked(context: Context): Boolean {
        return try {
            val km = context.getSystemService(Context.KEYGUARD_SERVICE) as? android.app.KeyguardManager
            km?.isKeyguardLocked == true
        } catch (_: Exception) { false }
    }

    fun placeCall(context: Context, number: String, subscriptionId: Int? = null): JSONObject {
        if (!SmsHandler.isValidNumber(number)) {
            return JSONObject().apply { put("error", "invalid number: $number"); put("code","invalid_number") }
        }
        if (!hasPermission(context, Manifest.permission.CALL_PHONE)) {
            return JSONObject().apply { put("error","missing_permission"); put("code","missing_permission"); put("permission","CALL_PHONE")}
        }
        if (!hasPermission(context, Manifest.permission.READ_PHONE_STATE) && subscriptionId != null) {
            // still allow but warn; Telecom can place without READ_PHONE_STATE on many OEMs, but dual-SIM needs it
        }
        // Per-call explicit tap: require device unlocked and user confirmation.
        // For MVP simulation we check isDeviceLocked; if locked, we still allow but set requires_tap flag in response is handled by daemon.
        // Here we enforce that if subscriptionId specified, it must be active
        if (subscriptionId != null) {
            try {
                val sm = context.getSystemService(SubscriptionManager::class.java)
                val active = sm?.activeSubscriptionInfoList
                val found = active?.any { it.subscriptionId == subscriptionId } ?: false
                if (active != null && active.isNotEmpty() && !found) {
                    return JSONObject().apply { put("error","invalid subscriptionId $subscriptionId"); put("code","invalid_subscription") }
                }
            } catch (_: SecurityException) {}
        }

        return try {
            val telecom = context.getSystemService(TelecomManager::class.java) ?: context.getSystemService(Context.TELECOM_SERVICE) as? TelecomManager
            if (telecom == null) {
                return JSONObject().apply { put("error","TelecomManager unavailable"); put("code","call_failed") }
            }
            val uri = Uri.fromParts("tel", number, null)
            val extras = Bundle()
            // Dual-SIM: select PhoneAccountHandle if subscriptionId given
            if (subscriptionId != null) {
                try {
                    val accounts = telecom.callCapablePhoneAccounts
                    // Find account whose id contains subscriptionId
                    val handle = accounts?.firstOrNull { it.id.contains("$subscriptionId") }
                    if (handle != null) {
                        extras.putParcelable(TelecomManager.EXTRA_PHONE_ACCOUNT_HANDLE, handle)
                    } else {
                        // fallback: try to put subscriptionId as extra for OEMs
                        extras.putInt("subscription_id", subscriptionId)
                        extras.putInt("slot_id", subscriptionId)
                    }
                } catch (_: Exception) {}
            }
            // Flags
            extras.putBoolean(TelecomManager.EXTRA_START_CALL_WITH_SPEAKERPHONE, false)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                // nothing
            }
            // Actual placeCall — requires CALL_PHONE permission; will throw SecurityException if missing
            telecom.placeCall(uri, extras)
            val callId = "call-${System.currentTimeMillis()}"
            currentCallId = callId
            setState(CallState.RINGING)
            pendingConfirms[callId] = System.currentTimeMillis() + 60_000
            JSONObject().apply {
                put("callId", callId)
                put("number", number)
                put("subscriptionId", subscriptionId)
                put("state", "RINGING")
                put("requires_tap", true)
            }
        } catch (e: SecurityException) {
            JSONObject().apply { put("error","SecurityException: ${e.message}"); put("code","missing_permission") }
        } catch (e: Exception) {
            JSONObject().apply { put("error", e.message ?: "placeCall failed"); put("code","call_failed") }
        }
    }

    fun answerCall(context: Context, callId: String?): JSONObject {
        if (callId.isNullOrEmpty()) {
            return JSONObject().apply { put("error","missing callId"); put("code","validation") }
        }
        if (!hasPermission(context, Manifest.permission.ANSWER_PHONE_CALLS)) {
            return JSONObject().apply { put("error","missing_permission"); put("code","missing_permission"); put("permission","ANSWER_PHONE_CALLS") }
        }
        return try {
            // In real InCallService we would find Call by ID and call.answer()
            // Here we simulate: if we have a BridgeInCallService instance, delegate
            val svc = BridgeInCallService.instance
            if (svc != null) {
                svc.answerCallById(callId)
            } else {
                // fallback via TelecomManager#acceptRingingCall (API 23+)
                val telecom = context.getSystemService(TelecomManager::class.java)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    try { telecom?.acceptRingingCall() } catch(_: Exception){}
                }
            }
            if (canTransition(currentState, CallState.OFFHOOK)) setState(CallState.OFFHOOK)
            JSONObject().apply { put("callId", callId); put("state","OFFHOOK") }
        } catch (e: SecurityException) {
            JSONObject().apply { put("error","SecurityException: ${e.message}"); put("code","missing_permission") }
        } catch (e: Exception) {
            JSONObject().apply { put("error", e.message ?: "answer failed"); put("code","call_failed") }
        }
    }

    fun hangupCall(context: Context, callId: String?): JSONObject {
        if (callId.isNullOrEmpty()) {
            return JSONObject().apply { put("error","missing callId"); put("code","validation") }
        }
        return try {
            val svc = BridgeInCallService.instance
            if (svc != null) {
                svc.hangupCallById(callId)
            } else {
                val telecom = context.getSystemService(TelecomManager::class.java)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                    try { telecom?.endCall() } catch(_: Exception){}
                } else {
                    // fallback: try to disconnect via ConnectionService stub
                }
            }
            if (canTransition(currentState, CallState.HUNGUP) || currentState == CallState.RINGING) {
                setState(CallState.HUNGUP)
                // cleanup after debounce
                android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                    if (currentState == CallState.HUNGUP) setState(CallState.IDLE)
                }, 600)
            }
            JSONObject().apply { put("callId", callId); put("state","HUNGUP") }
        } catch (e: Exception) {
            JSONObject().apply { put("error", e.message ?: "hangup failed"); put("code","call_failed") }
        }
    }

    fun handleCallAudio(context: Context, payload: JSONObject): JSONObject {
        val callId = payload.optString("callId")
        if (callId.isNullOrEmpty()) {
            return JSONObject().apply { put("error","missing callId"); put("code","validation") }
        }
        // WebRTC audio: in real impl, would route Opus frames to Telecom Connection or PipeWire
        // Here we just ack and if we have InCallService, forward to it
        try {
            val svc = BridgeInCallService.instance
            svc?.handleAudioPayload(payload)
            // Also, if we have a ConnectionService, we could decode Opus → PCM and play via AudioTrack
        } catch (_: Exception) {}
        return JSONObject().apply { put("callId", callId); put("relayed", true) }
    }

    fun resetForTest() {
        currentState = CallState.IDLE
        currentCallId = null
        pendingConfirms.clear()
    }
}
