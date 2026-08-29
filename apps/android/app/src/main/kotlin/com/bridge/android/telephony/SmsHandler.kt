package com.bridge.android.telephony

import android.Manifest
import android.app.PendingIntent
import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.database.Cursor
import android.net.Uri
import android.os.Build
import android.provider.Telephony
import android.telephony.SmsManager
import android.telephony.SubscriptionManager
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject

data class SmsMessage(
    val id: String,
    val address: String,
    val body: String,
    val date: Long,
    val type: Int,
    val read: Int,
    val subscriptionId: Int?
)

object SmsHandler {

    fun isValidNumber(n: String): Boolean {
        val digits = n.filter { it.isDigit() }
        return digits.length in 7..15 && n.trim().all { it.isDigit() || it == '+' || it == ' ' || it == '-' || it == '(' || it == ')' } && n.trim().isNotEmpty()
    }

    fun isValidBody(s: String): Boolean {
        return s.isNotEmpty() && s.length <= 918 && s.trim().isNotEmpty()
    }

    fun hasPermission(context: Context, perm: String): Boolean {
        return ContextCompat.checkSelfPermission(context, perm) == PackageManager.PERMISSION_GRANTED
    }

    fun isDeviceLocked(context: Context): Boolean {
        return try {
            val km = context.getSystemService(Context.KEYGUARD_SERVICE) as? android.app.KeyguardManager
            km?.isKeyguardLocked == true || km?.isDeviceLocked == true
        } catch (_: Exception) { false }
    }

    fun getActiveSubscriptions(context: Context): JSONArray {
        val arr = JSONArray()
        if (!hasPermission(context, Manifest.permission.READ_PHONE_STATE)) return arr
        try {
            val sm = context.getSystemService(SubscriptionManager::class.java) ?: return arr
            @Suppress("MissingPermission")
            val list = sm.activeSubscriptionInfoList ?: return arr
            for (info in list) {
                val obj = JSONObject()
                obj.put("id", info.subscriptionId)
                obj.put("displayName", info.displayName?.toString() ?: "SIM ${info.subscriptionId}")
                obj.put("carrier", info.carrierName?.toString() ?: "")
                obj.put("iccId", info.iccId ?: "")
                arr.put(obj)
            }
        } catch (e: SecurityException) {
            // permission denied
        } catch (_: Exception) {}
        return arr
    }

    fun listInbox(context: Context, limit: Int = 50, offset: Int = 0, subscriptionId: Int? = null): JSONObject {
        // Threat model: SMS preview requires unlock
        if (isDeviceLocked(context)) {
            return JSONObject().apply {
                put("error", "device_locked")
                put("code", "device_locked")
                put("message", "SMS preview requires device unlock")
            }
        }
        if (!hasPermission(context, Manifest.permission.READ_SMS)) {
            return JSONObject().apply {
                put("error", "missing_permission")
                put("code", "missing_permission")
                put("permission", "READ_SMS")
            }
        }
        val result = JSONObject()
        val messages = JSONArray()
        var resolver: ContentResolver? = null
        var cursor: Cursor? = null
        try {
            resolver = context.contentResolver
            val uri: Uri = Telephony.Sms.Inbox.CONTENT_URI // content://sms/inbox
            // Also support content://sms/ for broader but filtered
            val projection = arrayOf("_id", "address", "body", "date", "type", "read", "sub_id")
            val sortOrder = "date DESC LIMIT $limit OFFSET $offset"
            // Some OEMs don't support LIMIT in query; we handle manually if needed
            cursor = try {
                resolver.query(uri, projection, null, null, sortOrder)
            } catch (_: Exception) {
                // fallback without LIMIT
                resolver.query(uri, projection, null, null, "date DESC")
            }
            cursor?.let { c ->
                val idxId = c.getColumnIndex("_id")
                val idxAddr = c.getColumnIndex("address")
                val idxBody = c.getColumnIndex("body")
                val idxDate = c.getColumnIndex("date")
                val idxType = c.getColumnIndex("type")
                val idxRead = c.getColumnIndex("read")
                val idxSub = c.getColumnIndex("sub_id")
                var count = 0
                var skipped = 0
                while (c.moveToNext()) {
                    if (skipped < offset) { skipped++; continue }
                    if (count >= limit) break
                    val subId = if (idxSub >= 0) c.getInt(idxSub) else null
                    if (subscriptionId != null && subId != null && subId != subscriptionId) continue
                    val obj = JSONObject()
                    obj.put("id", if (idxId >= 0) c.getString(idxId) ?: "${System.currentTimeMillis()}_$count" else "${System.currentTimeMillis()}_$count")
                    obj.put("address", if (idxAddr >= 0) c.getString(idxAddr) ?: "" else "")
                    obj.put("body", if (idxBody >= 0) c.getString(idxBody) ?: "" else "")
                    obj.put("date", if (idxDate >= 0) c.getLong(idxDate) else System.currentTimeMillis())
                    obj.put("type", if (idxType >= 0) c.getInt(idxType) else 1)
                    obj.put("read", if (idxRead >= 0) c.getInt(idxRead) else 0)
                    if (idxSub >= 0 && !c.isNull(idxSub)) obj.put("subscriptionId", c.getInt(idxSub))
                    messages.put(obj)
                    count++
                }
            }
        } catch (e: SecurityException) {
            return JSONObject().apply { put("error", "SecurityException: ${e.message}"); put("code","missing_permission") }
        } catch (e: Exception) {
            return JSONObject().apply { put("error", e.message ?: "query failed"); put("code","sms_failed") }
        } finally {
            try { cursor?.close() } catch(_: Exception){}
        }
        result.put("messages", messages)
        result.put("subscriptions", getActiveSubscriptions(context))
        result.put("limit", limit)
        result.put("offset", offset)
        return result
    }

    fun sendSms(context: Context, address: String, body: String, subscriptionId: Int? = null): JSONObject {
        if (!isValidNumber(address)) {
            return JSONObject().apply { put("error", "invalid number: $address"); put("code","invalid_number") }
        }
        if (!isValidBody(body)) {
            return JSONObject().apply { put("error", "invalid body len ${body.length}"); put("code","invalid_body") }
        }
        if (!hasPermission(context, Manifest.permission.SEND_SMS)) {
            return JSONObject().apply { put("error","missing_permission"); put("code","missing_permission"); put("permission","SEND_SMS") }
        }
        // Validate subscription
        if (subscriptionId != null) {
            val active = getActiveSubscriptions(context)
            var found = false
            for (i in 0 until active.length()) {
                if (active.getJSONObject(i).optInt("id") == subscriptionId) { found = true; break }
            }
            if (!found && active.length() > 0) {
                return JSONObject().apply { put("error","invalid subscriptionId $subscriptionId"); put("code","invalid_subscription") }
            }
        }
        return try {
            val smsManager: SmsManager = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                context.getSystemService(SmsManager::class.java) ?: SmsManager.getDefault()
            } else {
                SmsManager.getDefault()
            }.let { default ->
                if (subscriptionId != null && Build.VERSION.SDK_INT >= 22) {
                    try {
                        SmsManager.getSmsManagerForSubscriptionId(subscriptionId)
                    } catch (_: Exception) { default }
                } else default
            }
            // Create pending intents for sent/delivery (optional)
            val sentIntent = try {
                PendingIntent.getBroadcast(context, 0, Intent("SMS_SENT"), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
            } catch (_: Exception) { null }
            val deliveryIntent = try {
                PendingIntent.getBroadcast(context, 0, Intent("SMS_DELIVERED"), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
            } catch (_: Exception) { null }

            // Handle long messages: divide if needed (SmsManager handles automatically but we log)
            if (body.length > 160) {
                val parts = smsManager.divideMessage(body)
                if (parts.size > 1) {
                    smsManager.sendMultipartTextMessage(address, null, parts, null, null)
                } else {
                    smsManager.sendTextMessage(address, null, body, sentIntent, deliveryIntent)
                }
            } else {
                smsManager.sendTextMessage(address, null, body, sentIntent, deliveryIntent)
            }
            JSONObject().apply {
                put("status", "sent")
                put("address", address)
                put("body_len", body.length)
                put("subscriptionId", subscriptionId)
                put("id", "sms-${System.currentTimeMillis()}")
            }
        } catch (e: SecurityException) {
            JSONObject().apply { put("error", "SecurityException: ${e.message}"); put("code","missing_permission") }
        } catch (e: IllegalArgumentException) {
            JSONObject().apply { put("error", "invalid arg: ${e.message}"); put("code","invalid_number") }
        } catch (e: Exception) {
            JSONObject().apply { put("error", e.message ?: "send failed"); put("code","sms_failed") }
        }
    }
}
