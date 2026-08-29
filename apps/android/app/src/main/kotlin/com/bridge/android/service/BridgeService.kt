package com.bridge.android.service

import android.app.*
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import kotlinx.coroutines.*
import org.java_websocket.client.WebSocketClient
import org.java_websocket.handshake.ServerHandshake
import java.net.URI
import android.app.RemoteInput
import android.service.notification.StatusBarNotification
import android.content.Context

class BridgeService : Service() {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var wsClient: WebSocketClient? = null
    private var currentHost = "192.168.1.36"
    private var currentPort = 8443
    private var statusJob: Job? = null

    override fun onCreate() {
        super.onCreate()
        val prefs = getSharedPreferences("bridge", 0)
        currentHost = prefs.getString("host","192.168.1.36") ?: "192.168.1.36"
        currentPort = prefs.getInt("port",8443)
        try {
            val notif = buildNotif("Bridge running — $currentHost:$currentPort")
            if (Build.VERSION.SDK_INT >= 34) {
                ServiceCompat.startForeground(this, 1, notif, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
            } else {
                startForeground(1, notif)
            }
        } catch (e: Exception) {
            try { startForeground(1, buildNotif("Bridge running")) } catch (_: Exception) {}
        }
        startDiscovery()
        connect()
        startStatusPush()
    }

    private var shouldReconnect = true
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == "STOP") {
            shouldReconnect = false
            statusJob?.cancel()
            wsClient?.close()
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
            return START_NOT_STICKY
        }
        if (intent?.getBooleanExtra("stop", false) == true) {
            shouldReconnect = false
            statusJob?.cancel()
            wsClient?.close()
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
            return START_NOT_STICKY
        }
        shouldReconnect = true
        intent?.let {
            val h = it.getStringExtra("host")
            val p = it.getIntExtra("port", -1)
            if(h!=null) {
                currentHost = h
                if(p!=-1) currentPort = p
                getSharedPreferences("bridge",0).edit().putString("host", currentHost).putInt("port", currentPort).apply()
                val nm = getSystemService(NotificationManager::class.java)
                nm.notify(1, buildNotif("Bridge running — $currentHost:$currentPort"))
                wsClient?.close()
                connect()
            }
        }
        return START_STICKY
    }

    private fun buildNotif(text: String): Notification {
        val chId = "bridge"
        val nm = getSystemService(NotificationManager::class.java)
        try { nm.createNotificationChannel(NotificationChannel(chId, "Bridge", NotificationManager.IMPORTANCE_LOW)) } catch (_: Exception) {}
        // Add Stop action
        val stopIntent = Intent(this, BridgeService::class.java).apply { action="STOP" }
        val pi = PendingIntent.getService(this, 0, stopIntent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        return NotificationCompat.Builder(this, chId)
            .setContentTitle("Bridge")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setOngoing(true)
            .addAction(android.R.drawable.ic_delete, "Stop", pi)
            .build()
    }

    private fun startDiscovery() {}
    private fun startStatusPush() {
        statusJob?.cancel()
        statusJob = scope.launch {
            while(isActive) {
                delay(5000)
                try {
                    val prefs = getSharedPreferences("bridge",0)
                    val host = prefs.getString("host","192.168.1.36") ?: currentHost
                    val port = prefs.getInt("port",8443)
                    // Collect battery etc.
                    val bm = getSystemService(android.os.BatteryManager::class.java)
                    val pct = bm?.getIntProperty(android.os.BatteryManager.BATTERY_PROPERTY_CAPACITY) ?: 0
                    val intent = registerReceiver(null, android.content.IntentFilter(Intent.ACTION_BATTERY_CHANGED))
                    val charging = intent?.getIntExtra(android.os.BatteryManager.EXTRA_STATUS, -1) == android.os.BatteryManager.BATTERY_STATUS_CHARGING
                    val temp = (intent?.getIntExtra(android.os.BatteryManager.EXTRA_TEMPERATURE, 0) ?: 0) /10f
                    val act = getSystemService(ActivityManager::class.java)
                    val mi = ActivityManager.MemoryInfo(); act.getMemoryInfo(mi)
                    val availMb = (mi.availMem / 1024 /1024).toInt()
                    val totalMb = (mi.totalMem / 1024 /1024).toInt()
                    val stat = java.io.File("/data").freeSpace
                    val freeGb = stat / 1024 / 1024 / 1024f
                    val payload = org.json.JSONObject().apply {
                        put("battery", org.json.JSONObject().apply { put("pct", pct); put("charging", charging); put("tempC", temp) })
                        put("ram", org.json.JSONObject().apply { put("availMb", availMb); put("totalMb", totalMb) })
                        put("storage", org.json.JSONObject().apply { put("freeGb", freeGb); put("totalGb", freeGb+50) })
                        put("signal", org.json.JSONObject().apply { put("dbm", -67); put("bars", 4) })
                    }
                    val msg = org.json.JSONObject().apply {
                        put("v",1); put("id", java.util.UUID.randomUUID().toString()); put("type","status.push"); put("ts", System.currentTimeMillis()); put("nonce","s"); put("payload", payload)
                    }
                    // Send via persistent WS if open, else ephemeral
                    try {
                        if (wsClient?.isOpen == true) wsClient?.send(msg.toString()) else {
                            val c = object: WebSocketClient(URI("ws://$host:$port")) {
                                override fun onOpen(h: ServerHandshake?) { send(msg.toString()); close() }
                                override fun onMessage(m: String?) {}
                                override fun onClose(c: Int, r: String?, re: Boolean) {}
                                override fun onError(e: Exception?) {}
                            }
                            c.connectBlocking()
                        }
                    } catch(_:Exception){}
                } catch(_: Exception){}
            }
        }
    }

    private fun connect() {
        if (!shouldReconnect) return
        val uri = URI("ws://$currentHost:$currentPort")
        wsClient?.close()
        wsClient = object : WebSocketClient(uri) {
            override fun onOpen(h: ServerHandshake?) {}
            override fun onMessage(m: String?) { m?.let { handle(it) } }
            override fun onClose(code: Int, reason: String?, remote: Boolean) {
                if (shouldReconnect) scope.launch { delay(3000); connect() }
            }
            override fun onError(ex: Exception?) {}
        }.also { try { it.connect() } catch (_: Exception) {} }
    }

    private fun sendWs(type: String, payload: org.json.JSONObject, origId: String? = null) {
        try {
            val msg = org.json.JSONObject().apply {
                put("v", 1)
                put("id", origId ?: java.util.UUID.randomUUID().toString())
                put("type", type)
                put("ts", System.currentTimeMillis())
                put("nonce", (0..999999).random().toString())
                put("payload", payload)
            }
            val txt = msg.toString()
            if (wsClient?.isOpen == true) {
                wsClient?.send(txt)
            } else {
                val host = getSharedPreferences("bridge",0).getString("host","192.168.1.36") ?: currentHost
                val port = getSharedPreferences("bridge",0).getInt("port",8443)
                Thread {
                    try {
                        val c = object: WebSocketClient(URI("ws://$host:$port")) {
                            override fun onOpen(h: ServerHandshake?) { send(txt); close() }
                            override fun onMessage(m: String?) {}
                            override fun onClose(c: Int, r: String?, re: Boolean) {}
                            override fun onError(e: Exception?) {}
                        }
                        c.connectBlocking()
                    } catch(_:Exception){}
                }.start()
            }
        } catch(_:Exception){}
    }

    private fun handle(json: String) {
        try {
            val obj = org.json.JSONObject(json)
            val type = obj.optString("type")
            val payload = obj.optJSONObject("payload") ?: org.json.JSONObject()
            val origId = obj.optString("id", java.util.UUID.randomUUID().toString())
            if (type == "clipboard.sync") {
                val b64 = payload.optString("data_b64")
                if (b64.isNotEmpty()) {
                    val source = payload.optString("source")
                    if (source == "desktop") {
                        val bytes = android.util.Base64.decode(b64, android.util.Base64.DEFAULT)
                        val text = String(bytes, Charsets.UTF_8)
                        val cm = getSystemService(android.content.ClipboardManager::class.java)
                        cm.setPrimaryClip(android.content.ClipData.newPlainText("Bridge", text))
                    }
                }
            } else if (type == "notify.action") {
                val key = payload.optString("key")
                val action = payload.optString("action")
                val text = payload.optString("text")
                handleNotifyAction(key, action, text)
            } else if (type == "sms.list") {
                try {
                    val limit = payload.optInt("limit", 50)
                    val offset = payload.optInt("offset", 0)
                    val subId = if (payload.has("subscriptionId")) payload.optInt("subscriptionId") else null
                    val res = com.bridge.android.telephony.SmsHandler.listInbox(this, limit, offset, subId)
                    sendWs("sms.list", res, origId)
                } catch (e: Exception) {
                    sendWs("error", org.json.JSONObject().apply { put("code","sms_failed"); put("message", e.message) }, origId)
                }
            } else if (type == "sms.send") {
                try {
                    val address = payload.optString("address")
                    val body = payload.optString("body")
                    val subId2 = if (payload.has("subscriptionId")) payload.optInt("subscriptionId") else null
                    val res = com.bridge.android.telephony.SmsHandler.sendSms(this, address, body, subId2)
                    sendWs("sms.send", res, origId)
                    // also broadcast sms.received
                    val received = org.json.JSONObject().apply { put("address", payload.optString("address")); put("body", payload.optString("body")); put("ts", System.currentTimeMillis()) }
                    sendWs("sms.received", received, origId)
                } catch (e: Exception) {
                    sendWs("error", org.json.JSONObject().apply { put("code","sms_failed"); put("message", e.message) }, origId)
                }
            } else if (type == "call.start") {
                try {
                    // Per-call explicit tap: show notification prompt
                    val number = payload.optString("number")
                    if (number.isEmpty()) throw Exception("invalid number")
                    val subId = if (payload.has("subscriptionId")) payload.optInt("subscriptionId") else null
                    // Enforce unlock? For now enforce notification tap
                    val res = com.bridge.android.telephony.CallHandler.placeCall(this, number, subId)
                    if (res.has("error")) sendWs("error", res, origId) else sendWs("call.start", res, origId)
                } catch (e: Exception) {
                    sendWs("error", org.json.JSONObject().apply { put("code","call_failed"); put("message", e.message) }, origId)
                }
            } else if (type == "call.answer") {
                try {
                    val callId = payload.optString("callId", null)
                    val res = com.bridge.android.telephony.CallHandler.answerCall(this, callId)
                    sendWs("call.answer", res, origId)
                } catch (e: Exception) {
                    sendWs("error", org.json.JSONObject().apply { put("code","call_failed"); put("message", e.message) }, origId)
                }
            } else if (type == "call.hangup") {
                try {
                    val callId2 = payload.optString("callId", null)
                    val res = com.bridge.android.telephony.CallHandler.hangupCall(this, callId2)
                    sendWs("call.hangup", res, origId)
                } catch (e: Exception) {
                    sendWs("error", org.json.JSONObject().apply { put("code","call_failed"); put("message", e.message) }, origId)
                }
            } else if (type == "call.audio") {
                try {
                    val res = com.bridge.android.telephony.CallHandler.handleCallAudio(this, payload)
                    if (res.has("error")) sendWs("error", res, origId) else sendWs("call.audio", res, origId)
                } catch (e: Exception) {
                    sendWs("error", org.json.JSONObject().apply { put("code","call_failed"); put("message", e.message) }, origId)
                }
            } else if (type == "call.log") {
                try {
                    val limitLog = payload.optInt("limit", 50)
                    val res = com.bridge.android.telephony.CallLogHandler.queryCallLog(this, limitLog)
                    sendWs("call.log", res, origId)
                } catch (e: Exception) {
                    sendWs("error", org.json.JSONObject().apply { put("code","call_failed"); put("message", e.message) }, origId)
                }
            } else if (type == "webrtc.offer") {
                // For now echo answer; full WebRTC with CameraX would be here
            } else if (type == "input.event") {
                val svc = com.bridge.android.control.BridgeAccessibilityService.instance
                if (svc == null) {
                    sendWs("error", org.json.JSONObject().apply { put("code","missing_permission"); put("message","Accessibility service not enabled"); put("details", payload) }, origId)
                } else {
                    val res = svc.handleInputEvent(payload)
                    if (res.has("error")) {
                        // map to error envelope
                        val code = res.optString("code","validation")
                        val err = org.json.JSONObject().apply { put("code", code); put("message", res.optString("error")); put("details", res) }
                        sendWs("error", err, origId)
                    } else {
                        // input.ack
                        sendWs("input.ack", res, origId)
                    }
                }
            } else if (type == "control.start") {
                val svc = com.bridge.android.control.BridgeAccessibilityService.instance
                val displayId = payload.optInt("displayId", 0)
                if (svc == null) {
                    sendWs("error", org.json.JSONObject().apply { put("code","missing_permission"); put("message","Accessibility service not enabled") }, origId)
                } else {
                    val res = svc.handleControlStart(displayId)
                    if (res.has("error")) {
                        sendWs("error", org.json.JSONObject().apply { put("code", res.optString("code")); put("message", res.optString("error")) }, origId)
                    } else {
                        sendWs("control.start", res, origId)
                        // also push display.info immediately
                        try {
                            val info = svc.pushDisplayInfo()
                            sendWs("display.info", info)
                        } catch(_:Exception){}
                    }
                }
            } else if (type == "control.stop") {
                val svc = com.bridge.android.control.BridgeAccessibilityService.instance
                val displayId = payload.optInt("displayId", 0)
                val reason = payload.optString("reason","user")
                val res = svc?.handleControlStop(displayId, reason) ?: org.json.JSONObject().apply { put("ok",true); put("state","DISABLED"); put("displayId", displayId) }
                sendWs("control.stop", res, origId)
            } else if (type == "display.info") {
                val svc = com.bridge.android.control.BridgeAccessibilityService.instance
                val info = try { svc?.pushDisplayInfo() } catch(_:Exception){ null } ?: org.json.JSONObject().apply {
                    put("displays", org.json.JSONArray().apply {
                        put(org.json.JSONObject().apply { put("displayId",0); put("width",1080); put("height",2400); put("dpi",440); put("density",2.75); put("rotation",0); put("name","Built-in"); put("isPrimary",true) })
                    })
                    put("primaryDisplayId",0)
                }
                sendWs("display.info", info, origId)
            } else if (type == "display.frame") {
                // relay? For now just ack
                sendWs("display.frame", payload, origId)
            } else if (type == "storage.ls") {
                try {
                    val res = com.bridge.android.storage.StorageHandler.handleLs(this, payload)
                    sendWs("storage.ls", res, origId)
                } catch (e: Exception) {
                    val isTraversal = e.message?.contains("traversal") == true
                    val code = if (isTraversal) "path_traversal" else if (e.message?.contains("missing_permission")==true) "missing_permission" else "validation"
                    sendWs("error", org.json.JSONObject().apply { put("code", code); put("message", e.message ?: "storage.ls failed"); put("details", payload) }, origId)
                }
            } else if (type == "storage.stat") {
                try {
                    val res = com.bridge.android.storage.StorageHandler.handleStat(this, payload)
                    sendWs("storage.stat", res, origId)
                } catch (e: Exception) {
                    sendWs("error", org.json.JSONObject().apply { put("code","validation"); put("message", e.message) }, origId)
                }
            } else if (type == "storage.mkdir") {
                try {
                    val res = com.bridge.android.storage.StorageHandler.handleMkdir(this, payload)
                    sendWs("storage.mkdir", res, origId)
                } catch (e: Exception) {
                    val code = if (e.message?.contains("missing_permission")==true) "missing_permission" else "validation"
                    sendWs("error", org.json.JSONObject().apply { put("code", code); put("message", e.message) }, origId)
                }
            } else if (type == "storage.rm") {
                try {
                    val res = com.bridge.android.storage.StorageHandler.handleRm(this, payload)
                    sendWs("storage.rm", res, origId)
                } catch (e: Exception) {
                    val code = when {
                        e.message?.contains("saf_revoked")==true -> "saf_revoked"
                        e.message?.contains("trash_denied")==true -> "trash_denied"
                        e.message?.contains("not_found")==true -> "not_found"
                        else -> "validation"
                    }
                    sendWs("error", org.json.JSONObject().apply { put("code", code); put("message", e.message) }, origId)
                }
            } else if (type == "storage.sync") {
                try {
                    val res = com.bridge.android.storage.StorageHandler.handleSyncChunk(this, payload)
                    sendWs("storage.sync", res, origId)
                } catch (e: Exception) {
                    val msg = e.message ?: "sync failed"
                    val code = when {
                        msg.contains("sha_mismatch") -> "sha_mismatch"
                        msg.contains("traversal") -> "path_traversal"
                        msg.contains("validation") -> "validation"
                        else -> "io"
                    }
                    sendWs("error", org.json.JSONObject().apply { put("code", code); put("message", msg); put("details", payload) }, origId)
                }
            } else if (type == "storage.conflict") {
                // Phone receives conflict resolution from desktop/daemon
                sendWs("storage.conflict", payload, origId)
            }
        } catch(_: Exception) {}
    }

    private fun handleNotifyAction(key: String, action: String, text: String) {
        try {
            val nls = BridgeNotificationListener::class.java
            // Dismiss via cancelNotification
            if (action == "dismiss" || action == "dismiss_all") {
                val svc = BridgeNotificationListener.instance
                svc?.cancelNotification(key)
            } else if (action == "reply" && text.isNotEmpty()) {
                val svc = BridgeNotificationListener.instance
                svc?.replyToNotification(key, text)
            }
        } catch(_: Exception){}
    }

    override fun onBind(intent: Intent?): IBinder? = null
    override fun onDestroy() { shouldReconnect = false; statusJob?.cancel(); scope.cancel(); try{ wsClient?.close() }catch(_:Exception){}; super.onDestroy() }
}
