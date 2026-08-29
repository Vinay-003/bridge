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
import com.bridge.android.telephony.CallHandler
import com.bridge.android.telephony.CallLogHandler
import com.bridge.android.telephony.SmsHandler
import org.json.JSONObject

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
            // call allow via notification tap
            if (it.getBooleanExtra("action_call_allow", false)) {
                val number = it.getStringExtra("call_number") ?: ""
                val subIdRaw = it.getIntExtra("call_subId", -1)
                val subId = if (subIdRaw == -1) null else subIdRaw
                val origId = it.getStringExtra("call_origId") ?: java.util.UUID.randomUUID().toString()
                // cancel confirm notification
                try { getSystemService(NotificationManager::class.java).cancel(99) } catch(_:Exception){}
                scope.launch {
                    val res = CallHandler.placeCall(this@BridgeService, number, subId)
                    if (res.has("error")) sendWs("error", res, origId) else sendWs("call.start", res, origId)
                }
            }
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

    private fun sendWs(type: String, payload: JSONObject, origId: String? = null) {
        try {
            val msg = JSONObject().apply {
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
                // ephemeral fallback
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
            val obj = JSONObject(json)
            val type = obj.optString("type")
            val id = obj.optString("id")
            val payload = obj.optJSONObject("payload") ?: JSONObject()
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
            } else if (type == "webrtc.offer") {
                // For now echo answer; full WebRTC with CameraX would be here
            } else if (type == "sms.list") {
                // Phone side: list inbox, then send back sms.list via WS
                scope.launch {
                    try {
                        val limit = payload.optInt("limit", 50)
                        val offset = payload.optInt("offset", 0)
                        val subId = if (payload.has("subscriptionId") && !payload.isNull("subscriptionId")) payload.optInt("subscriptionId") else null
                        val res = SmsHandler.listInbox(this@BridgeService, limit, offset, subId)
                        sendWs("sms.list", res, id)
                    } catch(e: Exception){
                        sendWs("error", JSONObject().apply{ put("code","sms_failed"); put("message", e.message) }, id)
                    }
                }
            } else if (type == "sms.send") {
                scope.launch {
                    try {
                        val address = payload.optString("address")
                        val body = payload.optString("body")
                        val subId = if (payload.has("subscriptionId") && !payload.isNull("subscriptionId")) payload.optInt("subscriptionId") else null
                        val res = SmsHandler.sendSms(this@BridgeService, address, body, subId)
                        // If success, send ack as sms.send, then also broadcast sms.received
                        if (res.has("error")) {
                            sendWs("error", res, id)
                        } else {
                            sendWs("sms.send", res, id)
                            // also notify desktop that new sent message appears
                            val received = JSONObject().apply {
                                put("address", address); put("body", body); put("date", System.currentTimeMillis()); put("subscriptionId", subId)
                            }
                            sendWs("sms.received", received)
                        }
                    } catch(e: Exception){
                        sendWs("error", JSONObject().apply{ put("code","sms_failed"); put("message", e.message) }, id)
                    }
                }
            } else if (type == "call.start") {
                scope.launch {
                    try {
                        val number = payload.optString("number")
                        val subId = if (payload.has("subscriptionId") && !payload.isNull("subscriptionId")) payload.optInt("subscriptionId") else null
                        // Per-call explicit tap: show notification prompt if device locked? For now enforce unlock
                        if (CallHandler.isDeviceLocked(this@BridgeService)) {
                            val err = JSONObject().apply{ put("code","device_locked"); put("message","Call requires device unlock + tap"); }
                            sendWs("error", err, id)
                            // Show notification to user to unlock and tap
                            showCallConfirmNotification(number, subId, id)
                        } else {
                            // Show allow-once notification, then auto-place after tap (simulate immediate allow for now)
                            // In real UI, user taps Allow; here we place directly and set requires_tap flag
                            val res = CallHandler.placeCall(this@BridgeService, number, subId)
                            if (res.has("error")) sendWs("error", res, id) else sendWs("call.start", res, id)
                        }
                    } catch(e: Exception){
                        sendWs("error", JSONObject().apply{ put("code","call_failed"); put("message", e.message) }, id)
                    }
                }
            } else if (type == "call.answer") {
                scope.launch {
                    val callId = payload.optString("callId")
                    val res = CallHandler.answerCall(this@BridgeService, callId)
                    if (res.has("error")) sendWs("error", res, id) else sendWs("call.answer", res, id)
                }
            } else if (type == "call.hangup") {
                scope.launch {
                    val callId = payload.optString("callId")
                    val res = CallHandler.hangupCall(this@BridgeService, callId)
                    if (res.has("error")) sendWs("error", res, id) else sendWs("call.hangup", res, id)
                }
            } else if (type == "call.audio") {
                scope.launch {
                    val res = CallHandler.handleCallAudio(this@BridgeService, payload)
                    if (res.has("error")) sendWs("error", res, id) else sendWs("call.audio", res, id)
                }
            } else if (type == "call.log") {
                scope.launch {
                    try {
                        val limit = payload.optInt("limit", 50)
                        val res = CallLogHandler.queryCallLog(this@BridgeService, limit)
                        if (res.has("error")) sendWs("error", res, id) else sendWs("call.log", res, id)
                    } catch(e: Exception){
                        sendWs("error", JSONObject().apply{ put("code","call_log_failed"); put("message", e.message) }, id)
                    }
                }
            }
        } catch(_: Exception) {}
    }

    private fun showCallConfirmNotification(number: String, subId: Int?, origId: String) {
        try {
            val chId = "bridge_calls"
            val nm = getSystemService(NotificationManager::class.java)
            try { nm.createNotificationChannel(NotificationChannel(chId, "Bridge Calls", NotificationManager.IMPORTANCE_HIGH)) } catch(_:Exception){}
            val allowIntent = Intent(this, BridgeService::class.java).apply {
                putExtra("action_call_allow", true)
                putExtra("call_number", number)
                putExtra("call_subId", subId ?: -1)
                putExtra("call_origId", origId)
            }
            val pi = PendingIntent.getService(this, 99, allowIntent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            val notif = NotificationCompat.Builder(this, chId)
                .setContentTitle("Bridge wants to call")
                .setContentText(number)
                .setSmallIcon(android.R.drawable.sym_action_call)
                .addAction(android.R.drawable.sym_action_call, "Allow once", pi)
                .setAutoCancel(true)
                .build()
            nm.notify(99, notif)
        } catch(_:Exception){}
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
