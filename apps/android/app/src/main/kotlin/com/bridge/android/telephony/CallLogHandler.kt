package com.bridge.android.telephony

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.database.Cursor
import android.provider.CallLog
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject

object CallLogHandler {

    fun hasPermission(context: Context): Boolean {
        return ContextCompat.checkSelfPermission(context, Manifest.permission.READ_CALL_LOG) == PackageManager.PERMISSION_GRANTED
    }

    fun isValidNumber(n: String): Boolean = SmsHandler.isValidNumber(n)

    fun queryCallLog(context: Context, limit: Int = 50): JSONObject {
        if (!hasPermission(context)) {
            return JSONObject().apply {
                put("error", "missing_permission")
                put("code", "missing_permission")
                put("permission", "READ_CALL_LOG")
            }
        }
        val result = JSONObject()
        val calls = JSONArray()
        var cursor: Cursor? = null
        try {
            val projection = arrayOf(CallLog.Calls.NUMBER, CallLog.Calls.TYPE, CallLog.Calls.DATE, CallLog.Calls.DURATION, CallLog.Calls.PHONE_ACCOUNT_ID, "sub_id")
            // Use try/catch for subscription_id column which may not exist on older
            val sortOrder = "${CallLog.Calls.DATE} DESC LIMIT $limit"
            cursor = try {
                context.contentResolver.query(CallLog.Calls.CONTENT_URI, null, null, null, sortOrder)
            } catch (_: Exception) {
                context.contentResolver.query(CallLog.Calls.CONTENT_URI, null, null, null, "${CallLog.Calls.DATE} DESC")
            }
            cursor?.let { c ->
                val idxNum = c.getColumnIndex(CallLog.Calls.NUMBER)
                val idxType = c.getColumnIndex(CallLog.Calls.TYPE)
                val idxDate = c.getColumnIndex(CallLog.Calls.DATE)
                val idxDur = c.getColumnIndex(CallLog.Calls.DURATION)
                val idxSub = c.getColumnIndex("subscription_id") // may be -1
                val idxSub2 = c.getColumnIndex(CallLog.Calls.PHONE_ACCOUNT_ID)
                var count = 0
                while (c.moveToNext() && count < limit) {
                    val number = if (idxNum >= 0) c.getString(idxNum) ?: "" else ""
                    val typeInt = if (idxType >= 0) c.getInt(idxType) else CallLog.Calls.OUTGOING_TYPE
                    val typeStr = when (typeInt) {
                        CallLog.Calls.INCOMING_TYPE -> "INCOMING"
                        CallLog.Calls.OUTGOING_TYPE -> "OUTGOING"
                        CallLog.Calls.MISSED_TYPE -> "MISSED"
                        CallLog.Calls.VOICEMAIL_TYPE -> "VOICEMAIL"
                        CallLog.Calls.REJECTED_TYPE -> "REJECTED"
                        CallLog.Calls.BLOCKED_TYPE -> "BLOCKED"
                        else -> "UNKNOWN"
                    }
                    val date = if (idxDate >= 0) c.getLong(idxDate) else System.currentTimeMillis()
                    val dur = if (idxDur >= 0) c.getLong(idxDur) else 0L
                    val obj = JSONObject()
                    obj.put("number", number)
                    obj.put("type", typeStr)
                    obj.put("date", date)
                    obj.put("duration", dur)
                    if (idxSub >= 0 && !c.isNull(idxSub)) {
                        obj.put("subscriptionId", c.getInt(idxSub))
                    } else if (idxSub2 >= 0 && !c.isNull(idxSub2)) {
                        // try to parse subscription from phone account id string like ".../SIM1..."
                        val acc = c.getString(idxSub2)
                        // keep as string hint
                        obj.put("phoneAccountId", acc)
                    }
                    calls.put(obj)
                    count++
                }
            }
        } catch (e: SecurityException) {
            return JSONObject().apply { put("error", "SecurityException: ${e.message}"); put("code","missing_permission") }
        } catch (e: Exception) {
            return JSONObject().apply { put("error", e.message ?: "query failed"); put("code","call_log_failed") }
        } finally {
            try { cursor?.close() } catch(_:Exception){}
        }
        result.put("calls", calls)
        result.put("limit", limit)
        return result
    }
}
